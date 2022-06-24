use crate::gh;

use anyhow::{anyhow, bail, Result};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

/// https://docs.github.com/ja/rest/reference/repos#get-a-repository
pub fn get_repos(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
) -> Result<Value> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}",
        owner.as_ref(),
        name.as_ref()
    ))?;
    gh::get_request(gh_token, &url, &[])
}

pub fn get_default_branch(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    memo: Option<&mut HashMap<String, String>>,
) -> Result<String> {
    let err_message = "Failed to parse the response to get the default branch";
    match memo {
        Some(memo) => {
            let key = format!("{}/{}", owner.as_ref(), name.as_ref());
            match memo.get(&key) {
                Some(default_branch) => Ok(default_branch.to_string()),
                None => {
                    let res = get_repos(gh_token, owner, name)?;
                    let default_branch = res
                        .get("default_branch")
                        .ok_or_else(|| anyhow!(err_message))?
                        .as_str()
                        .ok_or_else(|| anyhow!(err_message))?
                        .to_string();
                    memo.insert(key, default_branch.clone());
                    Ok(default_branch)
                }
            }
        }
        None => {
            let res = get_repos(gh_token, owner, name)?;
            Ok(res
                .get("default_branch")
                .ok_or_else(|| anyhow!(err_message))?
                .as_str()
                .ok_or_else(|| anyhow!(err_message))?
                .to_string())
        }
    }
}

/// https://docs.github.com/ja/rest/reference/branches#get-a-branch
pub fn get_branches(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch_name: impl AsRef<str>,
) -> Result<Value> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/branches/{}",
        owner.as_ref(),
        name.as_ref(),
        branch_name.as_ref()
    ))?;
    gh::get_request(gh_token, &url, &[])
}

pub fn get_latest_commit_sha(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch_name: impl AsRef<str>,
    memo: Option<&mut HashMap<String, String>>,
) -> Result<String> {
    let err_message = "Failed to parse the response to get a latest commit sha";
    match memo {
        Some(memo) => {
            let key = format!(
                "{}/{}/{}",
                owner.as_ref(),
                name.as_ref(),
                branch_name.as_ref()
            );
            match memo.get(&key) {
                Some(latest_commit_hash) => Ok(latest_commit_hash.to_string()),
                None => {
                    let res = get_branches(gh_token, owner, name, branch_name)?;
                    let latest_commit_hash = res
                        .get("commit")
                        .ok_or_else(|| anyhow!(err_message))?
                        .get("sha")
                        .ok_or_else(|| anyhow!(err_message))?
                        .as_str()
                        .ok_or_else(|| anyhow!(err_message))?
                        .to_string();
                    memo.insert(key, latest_commit_hash.clone());
                    Ok(latest_commit_hash)
                }
            }
        }
        None => {
            let res = get_branches(gh_token, owner, name, branch_name)?;
            Ok(res
                .get("commit")
                .ok_or_else(|| anyhow!(err_message))?
                .get("sha")
                .ok_or_else(|| anyhow!(err_message))?
                .as_str()
                .ok_or_else(|| anyhow!(err_message))?
                .to_string())
        }
    }
}

/// https://docs.github.com/ja/rest/reference/users#get-a-user
pub fn get_user(gh_token: impl AsRef<str>) -> Result<Value> {
    let url = Url::parse("https://api.github.com/user")?;
    gh::get_request(gh_token, &url, &[])
}

/// Return: (owner, name, affiliation)
pub fn get_author_info(gh_token: impl AsRef<str>) -> Result<(String, String, String)> {
    let res = get_user(gh_token)?;
    let err_message = "Failed to parse the response to get the author";
    let gh_account = res
        .get("login")
        .ok_or_else(|| anyhow!(err_message))?
        .as_str()
        .ok_or_else(|| anyhow!(err_message))?
        .to_string();
    let name = res
        .get("name")
        .ok_or_else(|| anyhow!(err_message))?
        .as_str()
        .ok_or_else(|| anyhow!(err_message))?
        .to_string();
    let affiliation = res
        .get("company")
        .ok_or_else(|| anyhow!(err_message))?
        .as_str()
        .ok_or_else(|| anyhow!(err_message))?
        .to_string();
    Ok((gh_account, name, affiliation))
}

/// https://docs.github.com/ja/rest/reference/repos#get-a-repository-readme
pub fn get_readme_url(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
) -> Result<Url> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/readme",
        owner.as_ref(),
        name.as_ref()
    ))?;
    let res = gh::get_request(gh_token, &url, &[])?;
    let err_message = "Failed to parse the response to get the README URL.";
    Ok(Url::parse(
        res.get("html_url")
            .ok_or_else(|| anyhow!(err_message))?
            .as_str()
            .ok_or_else(|| anyhow!(err_message))?,
    )?)
}

/// https://docs.github.com/ja/rest/reference/repos#get-repository-content
pub fn get_contents(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    path: impl AsRef<Path>,
    commit: impl AsRef<str>,
) -> Result<Value> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner.as_ref(),
        name.as_ref(),
        path.as_ref().display()
    ))?;
    gh::get_request(gh_token, &url, &[("ref", commit.as_ref())])
}

/// if called - path: src
/// return: src/main.rs, src/lib.rs, src/test.rs
pub fn get_file_list_recursive(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    path: impl AsRef<Path>,
    commit: impl AsRef<str>,
) -> Result<Vec<PathBuf>> {
    let res = get_contents(
        gh_token.as_ref(),
        owner.as_ref(),
        name.as_ref(),
        path,
        commit.as_ref(),
    )?;
    let err_message = "Failed to parse the response to get the file list.";
    match res.as_array() {
        Some(files) => {
            let mut file_list: Vec<PathBuf> = Vec::new();
            for file in files {
                let path = PathBuf::from(
                    file.get("path")
                        .ok_or_else(|| anyhow!(err_message))?
                        .as_str()
                        .ok_or_else(|| anyhow!(err_message))?,
                );
                let r#type = file
                    .get("type")
                    .ok_or_else(|| anyhow!(err_message))?
                    .as_str()
                    .ok_or_else(|| anyhow!(err_message))?;
                match r#type {
                    "file" => file_list.push(path),
                    "dir" => {
                        let mut sub_file_list = get_file_list_recursive(
                            gh_token.as_ref(),
                            owner.as_ref(),
                            name.as_ref(),
                            path,
                            commit.as_ref(),
                        )?;
                        file_list.append(&mut sub_file_list);
                    }
                    _ => {
                        unreachable!("Unknown file type: {}", r#type);
                    }
                }
            }
            Ok(file_list)
        }
        None => bail!(err_message),
    }
}

pub fn exists_branch(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch_name: impl AsRef<str>,
) -> Result<()> {
    match get_branches(&gh_token, &owner, &name, &branch_name) {
        Ok(_) => Ok(()),
        Err(err) => bail!("Branch {} does not exist: {}", branch_name.as_ref(), err),
    }
}

/// https://docs.github.com/en/rest/reference/git#get-a-reference
pub fn get_ref(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    r#ref: impl AsRef<str>,
) -> Result<Value> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/git/ref/{}",
        owner.as_ref(),
        name.as_ref(),
        r#ref.as_ref()
    ))?;
    gh::get_request(gh_token, &url, &[])
}

pub fn get_branch_sha(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch_name: impl AsRef<str>,
) -> Result<String> {
    let res = get_ref(
        gh_token.as_ref(),
        owner.as_ref(),
        name.as_ref(),
        format!("heads/{}", branch_name.as_ref()),
    )?;
    let err_message = "Failed to parse the response to get the branch sha.";
    Ok(res
        .get("object")
        .ok_or_else(|| anyhow!(err_message))?
        .get("sha")
        .ok_or_else(|| anyhow!(err_message))?
        .as_str()
        .ok_or_else(|| anyhow!(err_message))?
        .to_string())
}

/// https://docs.github.com/en/rest/reference/git#create-a-reference
pub fn create_ref(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    r#ref: impl AsRef<str>,
    sha: impl AsRef<str>,
) -> Result<Value> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/git/refs",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let body = json!({
        "ref": r#ref.as_ref(),
        "sha": sha.as_ref(),
    });
    gh::post_request(gh_token, &url, &body)
}

/// https://docs.github.com/en/rest/reference/git#update-a-reference
pub fn update_ref(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch_name: impl AsRef<str>,
    sha: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/git/refs/heads/{}",
        owner.as_ref(),
        name.as_ref(),
        branch_name.as_ref()
    ))?;
    let body = json!({
        "sha": sha.as_ref(),
    });
    gh::patch_request(gh_token, &url, &body)?;
    Ok(())
}

pub fn create_empty_branch(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch_name: impl AsRef<str>,
) -> Result<()> {
    let mut empty_contents: HashMap<PathBuf, String> = HashMap::new();

    let readme_content = r#"
# GA4GH Tool Registry Service (TRS) API generated by Yevis

Please see:

- [GitHub - sapporo-wes/yevis-cli](https://github.com/sapporo-wes/yevis-cli)
- [GA4GH - Tool Registry Service API](https://www.ga4gh.org/news/tool-registry-service-api-enabling-an-interoperable-library-of-genomics-analysis-tools/)
- [GitHub - ga4gh/tool-registry-service-schemas](https://github.com/ga4gh/tool-registry-service-schemas)
"#.to_string();

    empty_contents.insert(PathBuf::from("README.md"), readme_content);
    let empty_tree_sha = create_tree(&gh_token, &owner, &name, None::<String>, empty_contents)?;
    let empty_commit_sha = create_commit(
        &gh_token,
        &owner,
        &name,
        None::<String>,
        &empty_tree_sha,
        "Initial commit",
    )?;
    create_ref(
        &gh_token,
        &owner,
        &name,
        format!("refs/heads/{}", branch_name.as_ref()),
        &empty_commit_sha,
    )?;
    Ok(())
}

/// https://docs.github.com/en/rest/reference/git#create-a-tree
pub fn create_tree(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    base_tree: Option<impl AsRef<str>>,
    contents: HashMap<PathBuf, String>,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/git/trees",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let tree = contents
        .iter()
        .map(|(path, content)| {
            json!({
                "path": path.to_string_lossy().to_string(),
                "mode": "100644",
                "type": "blob",
                "content": content.as_str(),
            })
        })
        .collect::<Vec<_>>();
    let body = match base_tree {
        Some(base_tree) => {
            json!({
                "base_tree": base_tree.as_ref(),
                "tree": tree,
            })
        }
        None => {
            json!({
                "tree": tree,
            })
        }
    };
    let res = gh::post_request(gh_token, &url, &body)?;
    let err_message = "Failed to parse the response to create a tree.";
    Ok(res
        .get("sha")
        .ok_or_else(|| anyhow!(err_message))?
        .as_str()
        .ok_or_else(|| anyhow!(err_message))?
        .to_string())
}

/// https://docs.github.com/ja/rest/reference/git#create-a-commit
pub fn create_commit(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    parent: Option<impl AsRef<str>>,
    tree_sha: impl AsRef<str>,
    message: impl AsRef<str>,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/git/commits",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let body = match parent {
        Some(parent) => {
            json!({
                "tree": tree_sha.as_ref(),
                "parents": [parent.as_ref()],
                "message": message.as_ref(),
            })
        }
        None => {
            json!({
                "tree": tree_sha.as_ref(),
                "message": message.as_ref(),
            })
        }
    };
    let res = gh::post_request(gh_token, &url, &body)?;
    let err_message = "Failed to parse the response to create a commit.";
    Ok(res
        .get("sha")
        .ok_or_else(|| anyhow!(err_message))?
        .as_str()
        .ok_or_else(|| anyhow!(err_message))?
        .to_string())
}

/// https://docs.github.com/en/rest/reference/branches#sync-a-fork-branch-with-the-upstream-repository
pub fn merge_upstream(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/merge-upstream",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let body = json!({
        "branch": branch.as_ref(),
    });
    gh::post_request(gh_token, &url, &body)?;
    Ok(())
}

pub fn has_forked_repo(
    gh_token: impl AsRef<str>,
    user: impl AsRef<str>,
    ori_repo_owner: impl AsRef<str>,
    ori_repo_name: impl AsRef<str>,
) -> bool {
    let res = match gh::api::get_repos(&gh_token, &user, &ori_repo_name) {
        Ok(res) => res,
        Err(_) => return false,
    };
    match parse_fork_response(res) {
        Ok(fork) => match fork.fork {
            true => {
                fork.fork_parent.owner.as_str() == ori_repo_owner.as_ref()
                    && fork.fork_parent.name.as_str() == ori_repo_name.as_ref()
            }
            false => false,
        },
        Err(_) => false,
    }
}

struct Fork {
    pub fork: bool,
    pub fork_parent: ForkParent,
}

struct ForkParent {
    pub owner: String,
    pub name: String,
}

fn parse_fork_response(res: Value) -> Result<Fork> {
    let err_msg = "Failed to parse the response when getting repo info";
    let fork = res
        .get("fork")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_bool()
        .ok_or_else(|| anyhow!(err_msg))?;
    let fork_parent = res
        .get("parent")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?;
    let fork_parent_owner = fork_parent
        .get("owner")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("login")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    let fork_parent_name = fork_parent
        .get("name")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    Ok(Fork {
        fork,
        fork_parent: ForkParent {
            owner: fork_parent_owner.to_string(),
            name: fork_parent_name.to_string(),
        },
    })
}

/// https://docs.github.com/en/rest/reference/repos#create-a-fork
pub fn create_fork(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/forks",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let body = json!({});
    gh::post_request(gh_token, &url, &body)?;
    Ok(())
}

pub fn create_branch(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
    default_branch_sha: impl AsRef<str>,
) -> Result<()> {
    gh::api::create_ref(
        &gh_token,
        &owner,
        &name,
        format!("refs/heads/{}", branch.as_ref()),
        &default_branch_sha,
    )?;
    Ok(())
}

/// https://docs.github.com/en/rest/reference/repos#create-or-update-file-contents
pub fn create_or_update_file(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    path: impl AsRef<Path>,
    message: impl AsRef<str>,
    content: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<()> {
    let encoded_content = base64::encode(content.as_ref());
    let body = match get_contents_blob_sha(&gh_token, &owner, &name, &path, &branch) {
        Ok(blob) => {
            // If the file already exists, update it
            if blob.content == encoded_content {
                // If the file already exists and the content is the same, do nothing
                return Ok(());
            }
            json!({
                "message": message.as_ref(),
                "content": encoded_content,
                "sha": blob.sha,
                "branch": branch.as_ref()
            })
        }
        Err(e) => {
            // If the file does not exist, create it
            if e.to_string().contains("Not Found") {
                json!({
                    "message": message.as_ref(),
                    "content": encoded_content,
                    "branch": branch.as_ref()
                })
            } else {
                bail!(e)
            }
        }
    };

    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner.as_ref(),
        name.as_ref(),
        path.as_ref().display(),
    ))?;
    gh::put_request(&gh_token, &url, &body)?;
    Ok(())
}

struct Blob {
    pub content: String,
    pub sha: String,
}

/// https://docs.github.com/en/rest/reference/repos#get-repository-content
fn get_contents_blob_sha(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    path: impl AsRef<Path>,
    branch: impl AsRef<str>,
) -> Result<Blob> {
    let res = gh::api::get_contents(&gh_token, &owner, &name, &path, &branch)?;
    let err_msg = "Failed to parse the response when getting contents";
    let content = res
        .get("content")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    let sha = res
        .get("sha")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    Ok(Blob {
        content: content.to_string(),
        sha: sha.to_string(),
    })
}

/// https://docs.github.com/en/rest/reference/pulls#create-a-pull-request
/// head: the branch to merge from
/// base: the branch to merge into
///
/// return -> pull_request_url
pub fn post_pulls(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    title: impl AsRef<str>,
    head: impl AsRef<str>,
    base: impl AsRef<str>,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/pulls",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let body = json!({
        "title": title.as_ref(),
        "head": head.as_ref(),
        "base": base.as_ref(),
        "maintainer_can_modify": true
    });
    let res = gh::post_request(gh_token, &url, &body)?;
    let err_msg = "Failed to parse the response when positing pull request";
    Ok(res
        .get("url")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?
        .to_string())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::env;

    #[test]
    fn test_get_default_branch() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let branch = get_default_branch(&gh_token, "sapporo-wes", "yevis-cli", None)?;
        assert_eq!(branch, "main");
        Ok(())
    }

    #[test]
    fn test_get_default_branch_with_memo() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let mut memo = HashMap::new();
        get_default_branch(&gh_token, "sapporo-wes", "yevis-cli", Some(&mut memo))?;
        get_default_branch(&gh_token, "sapporo-wes", "yevis-cli", Some(&mut memo))?;
        Ok(())
    }

    #[test]
    fn test_get_latest_commit_sha() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        get_latest_commit_sha(&gh_token, "sapporo-wes", "yevis-cli", "main", None)?;
        Ok(())
    }

    #[test]
    fn test_get_latest_commit_sha_with_memo() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let mut memo = HashMap::new();
        get_latest_commit_sha(
            &gh_token,
            "sapporo-wes",
            "yevis-cli",
            "main",
            Some(&mut memo),
        )?;
        get_latest_commit_sha(
            &gh_token,
            "sapporo-wes",
            "yevis-cli",
            "main",
            Some(&mut memo),
        )?;
        Ok(())
    }

    #[test]
    #[cfg(not(tarpaulin))]
    fn test_get_author_info() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        get_author_info(&gh_token)?;
        Ok(())
    }

    #[test]
    fn test_get_readme_url() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let readme_url = get_readme_url(&gh_token, "sapporo-wes", "yevis-cli")?;
        assert_eq!(
            readme_url.to_string().as_str(),
            "https://github.com/sapporo-wes/yevis-cli/blob/main/README.md"
        );
        Ok(())
    }

    #[test]
    fn test_get_file_list_recursive() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let file_list =
            get_file_list_recursive(&gh_token, "sapporo-wes", "yevis-cli", ".", "main")?;
        assert!(file_list.contains(&PathBuf::from("README.md")));
        assert!(file_list.contains(&PathBuf::from("LICENSE")));
        assert!(file_list.contains(&PathBuf::from("src/main.rs")));
        Ok(())
    }

    #[test]
    fn test_get_file_list_recursive_with_dir() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let file_list =
            get_file_list_recursive(&gh_token, "sapporo-wes", "yevis-cli", "src", "main")?;
        assert!(file_list.contains(&PathBuf::from("src/main.rs")));
        Ok(())
    }
}
