use std::env;

use colored::*;
use git2::Error;
use git2::Oid;
use git2::Repository;

struct FileState {
    wt_add: usize,
    wt_edit: usize,
    wt_remove: usize,
    index_add: usize,
    index_edit: usize,
    index_remove: usize
}

impl FileState {

    fn wt_as_string(&self) -> String {
        FileState::as_string(self.wt_add, self.wt_edit, self.wt_remove)
    }

    fn index_as_string(&self) -> String {
        FileState::as_string(self.index_add, self.index_edit, self.index_remove)
    }

    fn as_string(add: usize, edit: usize, remove: usize) -> String {
        let mut result = "".to_owned();
        if add > 0 {
            result.push_str(&format!(" +{}", add));
        }
        if edit > 0 {
            result.push_str(&format!(" ~{}", edit));
        }
        if remove > 0 {
            result.push_str(&format!(" -{}", remove));
        }
        result
    }
}

struct BranchState {
    name: String,
    ahead: usize,
    behind: usize,
    is_detached: bool,
    sha: Oid
}

impl BranchState {
    fn as_string(&self) -> String {
        let mut result = format!("{}", self.name);
        if self.is_detached {
            result.push_str(&format!("({})", self.sha))
        }
        if self.behind > 0 {
            result.push_str(&format!(" ↓{}", self.behind))
        }
        if self.ahead > 0 {
            result.push_str(&format!(" ↑{}", self.ahead))
        }
        result
    }
}

fn main() -> Result<(), Error> {
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(error) => return add_context_to_error(Error::from_str(&error.to_string()), "Unable to get current dir")
    };
    match Repository::discover(current_dir) {
        Ok(repo) => {
            get_repo_info(&repo)?
        },
        Err(_) => { } // Not a git dir
    };

    Ok(())
}

fn get_repo_info(repo: &Repository) -> Result<(), Error> {
    print_bold_string("[".to_owned(), Color::Cyan);

    let branch_state = get_branch_info(&repo)?;
    print_bold_string(branch_state.as_string(), Color::Cyan);

    let file_state = get_file_state(&repo)?;
    let index_text = file_state.index_as_string();
    let wt_text = file_state.wt_as_string();

    if !index_text.is_empty() {
        print_bold_string(index_text, Color::Green);

        if !wt_text.is_empty() {
            print_bold_string(" |".to_owned(), Color::Cyan)
        }
    }

    if !wt_text.is_empty() {
        print_bold_string(wt_text, Color::Red);
    }

    print_bold_string("]".to_owned(), Color::Cyan);

    Ok(())
}

fn get_branch_info(repo: &Repository) -> Result<BranchState, Error> {

    if repo.is_empty()? {
        return get_empty_repo_branch_info(repo)
    }

    let head = match repo.head() {
        Ok(name) => name,
        Err(e) => return add_context_to_error(e, "Unable to get HEAD")
    };

    let head_fullname = match head.name() {
        Some(name) => name,
        None => return Err(Error::from_str("Unable to get local branch full name"))
    };
    let head_shortname = match head.shorthand() {
        Some(name) => name,
        None => return Err(Error::from_str("Unable to get local branch short name"))
    };

    let remote_name_buf = match repo.branch_upstream_name(head_fullname) {
        Ok(name) => name,

        // No remote branch
        Err(_) => return Ok(BranchState {
                                name: head_shortname.to_owned(),
                                ahead: 0,
                                behind: 0,
                                is_detached: repo.head_detached()?,
                                sha: head.peel_to_commit()?.id() })
    };

    let remote_reference = match remote_name_buf.as_str() {
        Some(name) => repo.find_reference(name)?,
        None => return Err(Error::from_str("Unable to get remote branch name"))
    };
    let upstream_oid = match remote_reference.target() {
        Some(id) => id,
        None => return Err(Error::from_str("Unable to get remote branch id"))
    };
    let local_oid = match head.target() {
        Some(id) => id,
        None => return Err(Error::from_str("Unable to get local branch id"))
    };

    let ahead_behind = repo.graph_ahead_behind(local_oid, upstream_oid)?;

    Ok(BranchState {
        name: head_shortname.to_owned(),
        ahead: ahead_behind.0,
        behind: ahead_behind.1,
        is_detached: repo.head_detached()?,
        sha: head.peel_to_commit()?.id() })
}

fn get_empty_repo_branch_info(repo: &Repository) -> Result<BranchState, Error> {
    let mut config = match repo.config() {
        Ok(mut live_config) => match live_config.snapshot() {
            Ok(snapshot) => snapshot,
            Err(e) => return add_context_to_error(e, "Unable to create config snapshot")
        },
        Err(e) => return add_context_to_error(e, "Unable to get repo config")
    };
    const DEFAULT_BRANCH_KEY: &str = "init.defaultBranch";
    let branch_name = match config.get_str(DEFAULT_BRANCH_KEY) {
        Ok(name) => name.to_string(),
        Err(_) => {
            let global = match config.open_global() {
                Ok(g) => g,
                Err(e) => return add_context_to_error(e, "Unable to get global config")
            };
            match global.get_str(DEFAULT_BRANCH_KEY) {
                Ok(name) => name.to_string(),
                Err(_) => "master".to_string()
            }
        }
    };
    return Ok(BranchState {
        name: branch_name,
        ahead: 0,
        behind: 0,
        is_detached: false,
        sha: Oid::zero()
    });
}

fn get_file_state(repo: &Repository) -> Result<FileState, Error> {
    let statuses = repo.statuses(None)?;

    let mut wt_add = 0;
    let mut wt_edit = 0;
    let mut wt_remove = 0;
    let mut index_add = 0;
    let mut index_edit = 0;
    let mut index_remove = 0;
    for status in statuses.iter().map(|s| s.status()) {
        if status.is_wt_new() {
            wt_add += 1;
        }
        if status.is_wt_modified() {
            wt_edit += 1;
        }
        if status.is_wt_deleted() {
            wt_remove += 1;
        }
        if status.is_wt_renamed() {
            wt_add += 1;
            wt_remove += 1;
        }
        if status.is_index_new() {
            index_add += 1;
        }
        if status.is_index_modified() {
            index_edit += 1;
        }
        if status.is_index_deleted() {
            index_remove += 1;
        }
        if status.is_index_renamed() {
            index_add += 1;
            index_remove += 1;
        }
    }

    Ok(FileState { wt_add, wt_edit, wt_remove, index_add, index_edit, index_remove })
}

fn print_bold_string(text: String, colour: Color) {
    print!("{}", text.color(colour).bold());
}

fn add_context_to_error<T>(e: Error, context: &str) -> Result<T, Error> {
    Err(Error::from_str(format!("{}. Error: {}", context, e).as_str()))
}
