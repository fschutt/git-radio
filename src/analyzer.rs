// src/analyzer.rs

use crate::model::*;
use git2::{Commit, Diff, DiffOptions, Repository};
use indicatif::ProgressBar;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::io::BufRead;
use std::path::Path;

pub fn analyze(repo_path: &Path) -> Result<AnalysisResult, git2::Error> {
    let repo = Repository::open(repo_path)?;
    println!("Analyzing repository at: {}", repo_path.display());

    // 1. Collect all commits and sort them chronologically
    let mut commits = Vec::new();
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        commits.push((oid, commit.time().seconds()));
    }
    commits.reverse(); // Walk from the first commit to the last

    let bar = ProgressBar::new(commits.len() as u64);
    bar.set_message("Analyzing commits");

    // --- Analysis State ---
    let file_map = RefCell::new(HashMap::new());
    let mut file_infos: Vec<FileInfo> = Vec::new();
    let mut next_file_id = 0;
    let mut change_map: ChangeMap = HashMap::new();
    let mut committer_map: HashMap<String, CommitterId> = HashMap::new();
    let mut committers: Vec<String> = Vec::new();

    let start_time = commits.first().map_or(0, |&(_, ts)| ts);
    let end_time = commits.last().map_or(0, |&(_, ts)| ts);

    // 2. Iterate through commits and process diffs
    for (i, (oid, _)) in commits.iter().enumerate() {
        let commit = repo.find_commit(*oid)?;

        let parent_tree = if i > 0 {
            let parent_commit = repo.find_commit(commits[i - 1].0)?;
            Some(parent_commit.tree()?)
        } else {
            None
        };
        let current_tree = commit.tree()?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(false);
        diff_opts.ignore_filemode(true);
        diff_opts.enable_fast_untracked_dirs(true);

        let mut diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&current_tree), Some(&mut diff_opts))?;
        diff.find_similar(None)?;
        
        process_diff(&diff, &commit, &repo, &file_map, &mut file_infos, &mut next_file_id, &mut change_map, &mut committer_map, &mut committers)?;

        bar.inc(1);
    }
    bar.finish_with_message("Analysis complete");

    Ok(AnalysisResult {
        files: file_infos,
        changes: change_map,
        committers,
        start_time,
        end_time,
        commits,
    })
}

fn process_diff<'a>(
    diff: &'a Diff<'a>,
    commit: &Commit,
    repo: &Repository,
    file_map: &RefCell<HashMap<String, FileId>>,
    file_infos: &mut Vec<FileInfo>,
    next_file_id: &mut FileId,
    change_map: &mut ChangeMap,
    committer_map: &mut HashMap<String, CommitterId>,
    committers: &mut Vec<String>,
) -> Result<(), git2::Error> {
    let commit_time = commit.time().seconds();
    let author = commit.author();
    let author_name = author.name().unwrap_or("Unknown").to_string();

    let committer_id = *committer_map.entry(author_name.clone()).or_insert_with(|| {
        let id = committers.len();
        committers.push(author_name);
        id
    });

    let current_line_no = Cell::new(0);

    let file_cb = &mut |delta: git2::DiffDelta, _| {
        let old_path = delta.old_file().path().and_then(|p| p.to_str()).map(String::from);
        let new_path = delta.new_file().path().and_then(|p| p.to_str()).map(String::from);
        let mut file_map = file_map.borrow_mut();

        match delta.status() {
            git2::Delta::Added => {
                if let Some(path) = new_path {
                    let id = *next_file_id;
                    *next_file_id += 1;
                    file_map.insert(path.clone(), id);
                    let blob = repo.find_blob(delta.new_file().id()).ok();
                    let line_count = blob.map_or(0, |b| b.content().lines().count());
                    let mut line_counts = BTreeMap::new();
                    line_counts.insert(commit_time, line_count);
                    file_infos.push(FileInfo { id, path, birth_time: commit_time, death_time: None, line_counts });
                }
            }
            git2::Delta::Deleted => {
                if let Some(path) = old_path {
                    if let Some(id) = file_map.remove(&path) {
                        if let Some(info) = file_infos.get_mut(id) {
                            info.death_time = Some(commit_time);
                        }
                    }
                }
            }
            git2::Delta::Renamed => {
                if let (Some(old), Some(new)) = (old_path, new_path) {
                    if let Some(id) = file_map.remove(&old) {
                        file_map.insert(new.clone(), id);
                        if let Some(info) = file_infos.get_mut(id) {
                            info.path = new;
                        }
                    }
                }
            }
            git2::Delta::Modified => {
                if let Some(path_str) = new_path {
                    if let Some(&file_id) = file_map.get(&path_str) {
                        let blob = repo.find_blob(delta.new_file().id()).ok();
                        let line_count = blob.map_or(0, |b| b.content().lines().count());
                        file_infos[file_id].line_counts.insert(commit_time, line_count);
                    }
                }
            }
            _ => {}
        }
        true
    };

    let hunk_cb = &mut |_delta: git2::DiffDelta, hunk: git2::DiffHunk| {
        if let Ok(hunk_header) = std::str::from_utf8(hunk.header()) {
            let parts: Vec<&str> = hunk_header.split(" @@").collect();
            if let Some(main_part) = parts.first() {
                let sub_parts: Vec<&str> = main_part.split(" +").collect();
                if sub_parts.len() > 1 {
                    if let Some(line_num_str) = sub_parts[1].split(',').next() {
                        current_line_no.set(line_num_str.parse::<usize>().unwrap_or(1));
                    }
                }
            }
        }
        true
    };

    let line_cb = &mut |delta: git2::DiffDelta, _hunk: Option<git2::DiffHunk>, line: git2::DiffLine| {
        let line_no = current_line_no.get();
        if let Some(path_str) = delta.new_file().path().and_then(|p| p.to_str()) {
            if let Some(&file_id) = file_map.borrow().get(path_str) {
                match line.origin() {
                    '+' | '-' => {
                        let history = change_map.entry((file_id, line_no)).or_default();
                        history.push(LineChange { timestamp: commit_time, committer_id });
                    }
                    _ => {}
                }
                if line.origin() != '-' {
                    current_line_no.set(line_no + 1);
                }
            }
        }
        true
    };

    diff.foreach(file_cb, None, Some(hunk_cb), Some(line_cb))?;

    Ok(())
}
