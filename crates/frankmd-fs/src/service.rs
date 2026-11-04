//! **[COMPAT]** facade with the exact semantics of the Rails `NotesService`
//! (plus the IO halves of `Note`, `Folder`, `HugoService`, `BacklinksService`),
//! taking raw string paths as the UI/workspace receives them.
//!
//! Every path goes through [`Vault::safe_path`]; every mutation holds the
//! exclusive filesystem lock; recents are updated through the optional
//! [`RecentStore`] exactly when Rails did (only when `.fed` is a regular file,
//! errors swallowed). The workspace layer adds DocId remapping on top.

use crate::recent::RecentStore;
use crate::{FsError, NoteFile, Vault};
use frankmd_core::Revision;
use frankmd_core::hugo::{GeneratedPost, HugoPathStyle};
use frankmd_core::tree::TreeNode;
use frankmd_core::wikilink::Backlink;

/// Rails `NotesService` over a [`Vault`].
pub struct NotesService<'a> {
    vault: &'a Vault,
    recents: Option<&'a dyn RecentStore>,
}

impl<'a> NotesService<'a> {
    /// Service without recents coupling.
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault, recents: None }
    }

    /// Couple delete/rename/delete_folder to the recents store.
    pub fn with_recents(mut self, store: &'a dyn RecentStore) -> Self {
        self.recents = Some(store);
        self
    }

    /// The vault.
    pub fn vault(&self) -> &Vault {
        self.vault
    }

    /// `list_tree` (shared lock; creates the lock file).
    pub fn list_tree(&self) -> Result<Vec<TreeNode>, FsError> {
        self.vault.list_tree()
    }

    /// `Folder#children`: the tree below `path` (`[]` when missing).
    pub fn folder_children(&self, path: &str) -> Result<Vec<TreeNode>, FsError> {
        todo!()
    }

    /// `read(path)`: exact bytes. `NotFound("Note not found: <path>")` unless
    /// it is a regular file (symlinks inside the vault are followed).
    pub fn read(&self, path: &str) -> Result<NoteFile, FsError> {
        todo!("backend.md §2.5")
    }

    /// `write(path, content)` (upsert): `mkdir -p` parent + atomic replace,
    /// keeping the existing mode.
    pub fn write(&self, path: &str, bytes: &[u8]) -> Result<Revision, FsError> {
        todo!("backend.md §2.5")
    }

    /// `create(path, content)`: no-clobber (a dangling symlink counts as
    /// existing) → `AlreadyExists("Destination already exists: <path>")`.
    pub fn create(&self, path: &str, bytes: &[u8]) -> Result<Revision, FsError> {
        todo!("backend.md §2.5")
    }

    /// `update(path, content)`: the file must exist (`NotFound`); parents are
    /// never created (HP-006).
    pub fn update(&self, path: &str, bytes: &[u8]) -> Result<Revision, FsError> {
        todo!("backend.md §2.5")
    }

    /// `delete(path, expected_revision:)`: `RevisionConflict` when the given
    /// revision is not the SHA-256 hex of the current bytes; removes recents.
    pub fn delete(&self, path: &str, expected_revision: Option<&str>) -> Result<(), FsError> {
        todo!("backend.md §2.5")
    }

    /// `rename(old, new)` for files **and** folders: source must exist
    /// (`NotFound("Note not found: <old>")`), destination must not
    /// (`AlreadyExists`), `mkdir -p` dest parent, no-clobber rename (EXDEV →
    /// copy + delete), remap recents. Never rewrites links in other notes.
    pub fn rename(&self, old: &str, new: &str) -> Result<(), FsError> {
        todo!("backend.md §2.5")
    }

    /// `Folder#rename`: [`NotesService::rename`], then rewrite the Hugo slug
    /// of `<new>/index.md` to `slugify(basename(new))` when its front matter
    /// has one (errors from the slug update are ignored).
    pub fn rename_folder(&self, old: &str, new: &str) -> Result<(), FsError> {
        todo!("folder.rb:271-326")
    }

    /// `create_folder(path)` (`mkdir -p` parents; `AlreadyExists` if present).
    pub fn create_folder(&self, path: &str) -> Result<(), FsError> {
        todo!()
    }

    /// `delete_folder(path)`: must be a directory (`NotFound("Folder not
    /// found: <path>")`) and empty incl. hidden files (`NotEmpty`); removes
    /// recents under it.
    pub fn delete_folder(&self, path: &str) -> Result<(), FsError> {
        todo!()
    }

    /// `exists?(path)` (follows symlinks; `false` for escaping paths is an
    /// error, as in Rails: `InvalidPath`).
    pub fn exists(&self, path: &str) -> Result<bool, FsError> {
        todo!()
    }

    /// `file?(path)`.
    pub fn is_file(&self, path: &str) -> Result<bool, FsError> {
        todo!()
    }

    /// `directory?(path)`.
    pub fn is_dir(&self, path: &str) -> Result<bool, FsError> {
        todo!()
    }

    /// `HugoService.template_source` IO half: the bytes of
    /// `<root>/.hugo_template.md` when it is a regular file (symlinks
    /// followed), `None` otherwise or on a read error (→ default template).
    pub fn hugo_template(&self) -> Option<Vec<u8>> {
        todo!("backend.md §4.3")
    }

    /// `HugoService.generate_blog_post` + `create`: build the post at `now`
    /// with the vault's template, then create it (no-clobber, parents made).
    pub fn create_hugo_post(
        &self,
        title: &str,
        parent: Option<&str>,
        style: HugoPathStyle,
        now: jiff::Zoned,
    ) -> Result<GeneratedPost, FsError> {
        todo!("backend.md §4")
    }

    /// `BacklinksService#find_backlinks(target, context_lines:)`: recursive
    /// walk (dot-entries skipped, **symlinks followed**, `.md` only), target
    /// skipped. `InvalidPath("Invalid path")` for `..` in the target. Order is
    /// unspecified (filesystem order); callers sort by `(path, line_number)`.
    pub fn backlinks(&self, target: &str, context_lines: usize) -> Result<Vec<Backlink>, FsError> {
        todo!("backend.md §5")
    }
}
