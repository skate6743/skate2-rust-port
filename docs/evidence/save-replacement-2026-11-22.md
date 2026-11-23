# The save's replacement, without backups — November 22, 2026

## What happened

Players reported, mostly on Linux and now and then on Windows, that the game could stop with an
error as it saved, and that its saves folder filled with `.bak` files.

## Why

- Every save of `SKATER.P` wrote the new profile to `SKATER.P.<pid>.<n>.tmp`, moved the old save
  aside as `SKATER.P.<pid>.<n>.bak`, then moved the new file into place
  (`EditSkaterProfileStore::commit`). `<pid>` is the process id and `<n>` a count of the run's
  saves from 0. The backup was never removed: a saves folder grew by the save's 4 MB with every
  save (one checked folder held 731 backups, 2.9 GB).
- The step that moved the old save aside refused a name already there (`SKATER.P backup path
  already exists`), and the temporary file was opened to fail on a name already there. Both
  names repeat as soon as a run gets a process id an earlier run had, since each run counts from
  0 again: its first save then fails, and each later one whose number an earlier run reached.
  Process ids repeat quickly on Linux (and nearly every launch inside a container's own process
  namespace) and in time on Windows. Seen in a saves folder: `SKATER.P.<pid>.0.bak` four hours
  older than the running game with that process id, whose first save it had refused, beside its
  `…1.bak`.
- A refused save is an error to its caller. The career's autosave logs it; leaving Options or
  Edit Skater and the fullscreen toggle hand it on, which ends the game.
- Between the two moves there was no `SKATER.P` at all, so a run that ended there left the save
  only under a backup's name.
- The backup step was there on the belief that a rename cannot replace a file on Windows. It
  can: `std::fs::rename` is `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` there and `rename(2)`
  elsewhere, and the port's other writers (the custom difficulty file, the career's photos)
  already replace their files that way.

## What the port does now

- A save writes the encoded, read-back profile to `SKATER.P.<pid>.tmp`, syncs it to the disk and
  renames it over `SKATER.P`. The save is whole at every moment, the old one until the rename and
  the new one from it. No backup is made, and nothing is left beside the save.
- The temporary file has one name per process, written over if a run that ended mid-save left it:
  a process's saves are one at a time (the store's file lock), so no count is needed and no name
  can be taken.
- On Windows a rename another program refuses for a moment (a virus scan or a synced folder
  holding the save: access denied, sharing violation, lock violation) is tried again every 15 ms
  for up to a second and a half. A rename that still fails leaves the old save untouched, removes
  the temporary file and is the error it was before.
- On Unix the save's directory is synced after the rename.
- Files earlier builds left (`SKATER.P.<pid>.<n>.bak`, `SKATER.P.<pid>.<n>.tmp`) are not read,
  not needed and not removed by the game. They can be deleted by hand; the newest `.bak` is the
  save as it was one save before the last by such a build.

## Verified

- A standalone copy of the write and the rename, built with the same compiler, on Windows and on
  Linux: a save over an existing save leaves the one file with the new bytes; a file under the
  process's temporary name and old `.bak`/`.tmp` names under its process id do not stop a save;
  a save goes through while a reader holds the file open. On Windows, with the save held without
  sharing for 200 ms the save went through after 214 ms, and with it held for good the save
  failed after 1.5 s with the old bytes in place and no temporary file left.
- The store's tests (`edit_skater_profile`): five saves in a row leave `SKATER.P` alone in its
  folder; backups and temporary files earlier runs left under this process's id, and a file under
  the name this process writes to, do not stop a save and the earlier ones stay as they were; a
  save that cannot take the save's name fails with nothing left beside the old one.
- In the game: Extras' Max Out Save on a scratch save root wrote the save (its bytes changed),
  the career loaded it back (413 challenges completed), and the folder held `SKATER.P` and
  nothing of the store's beside it.
