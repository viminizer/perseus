# Persistence Decisions (Locked)

Date: 2026-02-08

## Format and Structure
- Canonical on-disk format: Postman Collection v2.1.
- One collection per project root.
- Multiple projects live inside the collection.
- Requests are identified and stored by UUID.
- Filenames for requests are UUID-based (not name-based).

## Sidebar and Explorer
- Only one project is visible in the sidebar at a time.
- An always-visible project switcher appears at the top of the sidebar.
- Default sorting is alphabetical only.
- Reordering within a folder is not supported (j/k are navigation only).

## Keybinds (Confirmed)
- a: add request or folder ("name/child" creates folder + request; trailing "/" creates folder)
- m: move
- d: delete
- D: duplicate
- c: copy
- r: rename
- j/k: move selection up/down
- h: collapse folder or project
- l: expand folder or project; open request
- Enter: open request

## Keybinds (Added)
- /: sidebar search
- ?: open keybind help
- [: outdent (move to parent)
- ]: indent (move into folder)
- Shift+h: collapse all
- Shift+l: expand all

## Sidebar Width
- Default width: 320px
- Min width: 280px
- Max width: 420px
- Resizable drag handle
