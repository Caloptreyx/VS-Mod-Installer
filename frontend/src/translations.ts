import { defineTranslations } from 'shared';

const translations = defineTranslations({
  items: {},
  translations: {
    admin: {
      access: 'Access',
      eggs: 'Eggs',
      eggsDescription:
        'Servers of these eggs get the VS Mods page. Leave empty to detect Vintage Story servers by their egg, startup command and files.',
      inspection: 'Mod inspection',
      maxArchive: 'Largest mod archive read in full (MiB)',
      maxArchiveDescription:
        "To identify a disabled mod (or one Wings can't open), its archive is read in full to find modinfo.json. Larger archives are listed without details.",
      save: 'Save',
      saved: 'Settings saved.',
    },
    pages: {
      server: {
        vsMods: {
          title: 'VS Mods',
          tabs: {
            installed: 'Installed',
            browse: 'Browse',
          },
          environment: {
            unknownVersion: 'Unknown version',
            gameVersion: 'Vintage Story {version}',
            detectedFrom: 'Detected from {source}.',
            notDetected: 'Could not be detected.',
            source: {
              settings: 'the extension settings',
              egg: 'the server egg',
              startup: 'the startup command (--dataPath)',
              config: 'serverconfig.json (ModPaths)',
              default: "the game's default data path",
              files: 'the server files',
              log: 'the server log (server-main.log)',
              variables: 'the startup variables',
            },
            alert: {
              versionTitle: 'Game version could not be detected',
              versionContent:
                'Start the server once so it can be identified. Until then, check that the releases you install are made for your Vintage Story version.',
              noContentTitle: 'Mods cannot be identified',
              noContentContent:
                "You can see the mod files, but reading their contents needs the permission to read file contents. Without it, mods can't be matched to the ModDB and the game version can't be detected.",
              unsupportedTitle: 'Not a Vintage Story server',
              unsupportedContent:
                "This page installs Vintage Story mods, but this server doesn't look like a Vintage Story server: neither its egg, its startup command nor its files belong to Vintage Story.",
            },
          },
          gameVersions: {
            any: 'Any version',
          },
          side: {
            both: 'Client & Server',
            server: 'Server',
            client: 'Client only',
          },
          channel: {
            stable: 'Stable',
            prerelease: 'Pre-release',
          },
          compatibility: {
            exact: 'Compatible',
            same_major: 'Same major version',
            incompatible: 'Other game version',
            unknown: 'Unknown',
          },
          button: {
            install: 'Install',
            installed: 'Installed',
            update: 'Update',
            changeVersion: 'Change Version',
            viewDetails: 'View Details',
            dependencies: 'Dependencies',
            enable: 'Enable',
            disable: 'Disable',
            delete: 'Delete',
            refresh: 'Refresh',
            browse: 'Browse Mods',
            back: 'Back',
            openModDb: 'Open on ModDB',
          },
          toast: {
            installed: '{mod} {version} installed. Restart the server to load it.',
            changed: '{mod} changed to {version}. Restart the server to apply the change.',
            enabled: '{mod} enabled. Restart the server to apply the change.',
            disabled: '{mod} disabled. Restart the server to apply the change.',
            deleted: '{mod} deleted. Restart the server to apply the change.',
            dependenciesInstalled: 'Dependencies installed: {count}. Restart the server to load them.',
          },
          browse: {
            searchPlaceholder: 'Search the Vintage Story ModDB...',
            gameVersion: 'Game version',
            tag: 'Tag',
            anyTag: 'Any tag',
            sort: 'Sort by',
            showClient: 'Show client-only mods',
            showClientDescription: 'Client-only mods have no effect on a server; players install them themselves.',
            sortOptions: {
              relevance: 'Relevance',
              downloads: 'Downloads',
              follows: 'Followers',
              trending: 'Trending',
              updated: 'Recently updated',
              newest: 'Newest',
            },
            by: 'by {author}',
            downloads: '{count} downloads',
            followers: '{count} followers',
            updated: 'Updated',
            empty: {
              title: 'No mods found',
              description: 'Try a different search term or loosen the filters.',
            },
          },
          installed: {
            searchPlaceholder: 'Search installed mods...',
            filter: {
              all: 'All',
              enabled: 'Enabled',
              disabled: 'Disabled',
              updates: 'Updates ({count})',
            },
            summary: '{total} installed · {enabled} enabled · {disabled} disabled',
            unknownVersion: 'Unknown',
            notOnModDb: 'Not found on the ModDB',
            infoError: {
              permission: 'Needs the permission to read file contents to identify',
              code_mod: 'Code mod without modinfo.json',
              no_modinfo: 'No modinfo.json in this mod',
              invalid_modinfo: 'modinfo.json could not be read',
              invalid_archive: 'The archive could not be read',
              too_large: 'Too large to inspect',
              unreadable: 'Could not be read',
            },
            unmetDependencies: 'Needs: {mods}',
            updateAvailable: 'Update: {version}',
            checkingUpdates: 'Checking the ModDB for updates...',
            updatesFailed: 'Updates could not be checked: {error}',
            catalogFailed: "The ModDB could not be reached, so mods aren't matched to it: {error}",
            columns: {
              mod: 'Mod',
              version: 'Version',
              file: 'File',
              size: 'Size',
              enabled: 'Enabled',
            },
            empty: {
              title: 'No mods installed',
              description: 'Mods you install go to {path}. Browse the ModDB to find some.',
            },
            noMatches: 'No installed mods match your search.',
            modal: {
              delete: {
                title: 'Delete Mod',
                content: 'Are you sure you want to delete **{mod}**? The file `{file}` will be moved to the trash.',
              },
            },
          },
          details: {
            tabs: {
              description: 'Description',
              releases: 'Releases',
              screenshots: 'Screenshots',
            },
            installedVersion: 'Installed: {version}',
            installedDisabled: 'Installed (disabled): {version}',
            notFound: 'This mod could not be loaded from the ModDB.',
            stats: {
              downloads: 'Downloads',
              followers: 'Followers',
              updated: 'Updated',
              created: 'Created',
            },
            links: {
              homepage: 'Homepage',
              source: 'Source',
              issues: 'Issues',
              wiki: 'Wiki',
              trailer: 'Trailer',
            },
            releases: {
              search: 'Search',
              searchPlaceholder: 'Version or file name',
              gameVersion: 'Game version',
              channel: 'Channel',
              allChannels: 'All channels',
              columns: {
                version: 'Version',
                channel: 'Channel',
                gameVersions: 'Game versions',
                compatibility: 'This server',
                released: 'Released',
                downloads: 'Downloads',
              },
              current: 'Installed',
              recommended: 'Recommended for this server',
              changelog: 'Changelog',
              changelogTitle: 'Changelog of {version}',
              empty: 'No releases match these filters.',
            },
          },
          installModal: {
            title: 'Install {mod}',
            changeTitle: 'Change Version of {mod}',
            release: 'Release',
            recommendedHint: '★ marks the release picked for Vintage Story {version}.',
            forGame: 'for Vintage Story {versions}',
            noReleases: 'This mod has no release with a file that can be installed.',
            file: 'File',
            currentVersion: 'Currently installed: **{version}**. It will be replaced by the selected release.',
            alreadyInstalled: 'This release is already installed.',
            incompatible:
              'This release is made for Vintage Story {versions}, but your server runs {detected}. It may not load.',
            sameMajor:
              "This release lists Vintage Story {versions}, not {detected}. Releases usually keep working across patches of the same version, but it isn't guaranteed.",
            notDetected: "Your server's game version could not be detected. Make sure the release matches your server.",
            clientOnly: 'This mod only runs on the client. Installing it on the server has no effect.',
            submit: 'Install',
            submitChange: 'Change Version',
          },
          dependencies: {
            title: 'Dependencies of {mod}',
            intro: '{mod} needs other mods. Install the missing ones from the ModDB?',
            allMet: 'Everything {mod} needs is installed.',
            none: 'This mod only depends on the game itself.',
            state: {
              satisfied: 'Installed',
              builtin: 'Part of the game',
              game_too_old: 'Needs Vintage Story {version} or newer',
              disabled: 'Installed, but disabled',
              outdated: 'Needs {version} or newer, {installed} is installed',
              missing: 'Not installed',
            },
            problem: {
              not_on_moddb: 'not found on the ModDB, install it yourself',
              no_release: 'the ModDB has no release with a file for it',
              no_new_enough_release: 'the ModDB has no release of {version} or newer',
            },
            submit: 'Install Selected ({count})',
            close: 'Close',
          },
        },
      },
    },
  },
});

export const useExtTranslations = translations.useTranslations.bind(translations);
export const getExtTranslations = translations.getTranslations.bind(translations);

export default translations;
