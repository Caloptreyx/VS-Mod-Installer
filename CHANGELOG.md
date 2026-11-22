# Changelog

## 0.1.0

- First version: a **VS Mods** page for Vintage Story servers to browse, install, update, enable,
  disable and remove mods from the official ModDB.
- Browse with search, tag and game version filters, sorting and a client-only toggle; mod details
  with the sanitized description, screenshots and every release with its game versions.
- Release selection for the server's game version (stable first, then same major version), with a
  warning instead of a block for releases made for another version or for the client only.
- Installed list from each mod's `modinfo.json` (read like the game reads it: any key case,
  comments, trailing commas), matched to the ModDB by mod id, with available updates. An update
  that installs the new file but can't remove the old one says so and names the file.
- Dependencies from `modinfo.json` checked after an install and on the installed list; missing ones
  offered from the ModDB, one level deep, with the rest listed as unresolved.
- Detection of Vintage Story servers, the data path (`--dataPath`), the Mods folder (`ModPaths` in
  `serverconfig.json`) and the game version (`VintagestoryAPI.dll`, `.vs-version`,
  `server-main.log`, startup variables).
- Uses the panel's file permissions; installs through the panel's file pull endpoint. Admin settings
  for eggs and the largest archive read in full.
