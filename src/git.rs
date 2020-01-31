use git2::{BranchType, Error, StatusOptions};
pub use git2::{Oid, Repository};
use regex::Regex;

pub struct Git {
    pub repo: Repository,
    pub head_message: String,
    pub head_hash: String,
    pub branch_name: Option<String>,
    pub upstream: Option<String>,
}

impl Git {
    pub fn open() -> Result<Git, Error> {
        let repo = Repository::open(".")?;
        let head_message;
        let head_hash;
        let branch_name;
        let upstream;

        {
            let (object, maybe_ref) = repo.revparse_ext("HEAD")?;
            let commit = object.as_commit().unwrap();
            head_message = commit.message().unwrap().to_string();
            head_hash = hash_from_oid(object.id());
            branch_name = maybe_ref.and_then(|x| x.shorthand().map(|x| x.to_string()));
            upstream = if let Some(name) = branch_name.as_ref() {
                if let Ok(remote_branch) = repo
                    .find_branch(name, BranchType::Local)
                    .and_then(|x| x.upstream())
                {
                    remote_branch.name()?.map(|x| x.to_string())
                } else {
                    None
                }
            } else {
                None
            };
        }

        Ok(Git {
            repo,
            head_message,
            head_hash,
            branch_name,
            upstream,
        })
    }

    pub fn get_forked_hash(&self) -> Result<Option<String>, Error> {
        let re = Regex::new(r"(?m)^\s*Forked at:\s*(\w+)").unwrap();

        if let Some(captures) = &re.captures(self.head_message.as_str()) {
            Ok(Some(captures[1].to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn get_parent_branch(&self) -> Result<Option<String>, Error> {
        let re = Regex::new(r"(?m)^\s*Parent branch:\s*(\w+)").unwrap();

        if let Some(captures) = &re.captures(self.head_message.as_str()) {
            Ok(Some(captures[1].to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn get_unstaged_files(&self) -> Result<Vec<String>, Error> {
        let mut files = Vec::new();
        let mut options = StatusOptions::new();
        options.include_untracked(true);
        options.include_ignored(false);

        for entry in self.repo.statuses(Some(&mut options))?.iter() {
            files.push(entry.path().unwrap().to_string());
        }

        Ok(files)
    }

    pub fn branch(&self, name: &str) -> Result<String, Error> {
        let object = self.repo.revparse_single("HEAD")?;
        let commit = object.as_commit().unwrap();
        let branch = self.repo.branch(name, &commit, false)?;

        Ok(branch.get().name().unwrap().to_string())
    }

    pub fn set_head(&mut self, branch_name: &str) -> Result<(), Error> {
        let branch = self.repo.find_branch(branch_name, BranchType::Local)?;
        let object = self.repo.revparse_single(branch_name)?;

        self.repo.set_head(branch.get().name().unwrap())?;

        self.branch_name = Some(branch_name.to_string());
        self.head_hash = hash_from_oid(object.id());
        if let Ok(upstream) = branch.upstream() {
            self.upstream = upstream.name()?.map(|x| x.to_string());
        }

        Ok(())
    }

    pub fn reset_soft(&mut self, revision: &str, message: &str) -> Result<String, Error> {
        let object = self.repo.revparse_single(revision)?;
        let oid = object.id();

        if let (_, Some(mut reference)) = self.repo.revparse_ext(self.branch_name.as_ref().unwrap_or(&self.head_hash))? {
            reference.set_target(oid, message)?;
        } else {
            self.repo.set_head_detached(oid)?;
        }

        self.head_hash = hash_from_oid(oid);

        Ok(self.head_hash.clone())
    }

    pub fn commit(&mut self, message: &str) -> Result<Oid, Error> {
        let signature = self.repo.signature()?;
        let object = self.repo.revparse_single("HEAD")?;
        let commit = object.as_commit().unwrap();
        let tree = commit.tree()?;

        let oid = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&commit],
        )?;

        self.head_hash = hash_from_oid(oid);

        Ok(oid)
    }

    pub fn full_delete_branch(&self, branch_name: &str) -> Result<(), Error> {
        let mut local_branch = self.repo.find_branch(branch_name, BranchType::Local)?;

        if let Ok(mut remote_branch) = local_branch.upstream() {
            remote_branch.delete()?;
        }

        local_branch.delete()?;

        Ok(())
    }
}

pub fn hash_from_oid(oid: Oid) -> String {
    let slice = oid.as_bytes();
    let mut out = String::with_capacity(slice.len() * 2);

    for x in slice {
        out.push_str(format!("{:02x}", x).as_str());
    }

    out
}
