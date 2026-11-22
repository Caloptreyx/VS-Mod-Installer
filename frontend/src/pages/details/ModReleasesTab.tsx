import {
  faCheck,
  faDownload,
  faFileLines,
  faMagnifyingGlass,
  faRightLeft,
  faStar,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center } from '@mantine/core';
import { useEffect, useMemo, useState } from 'react';
import ActionIcon from '@/elements/buttons/ActionIcon.tsx';
import Button from '@/elements/buttons/Button.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Table, { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import Select from '@/elements/input/Select.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import { Modal } from '@/elements/modals/Modal.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import type { InstalledMod, Release } from '../../lib/schemas.ts';
import { ChannelBadge, CompatibilityBadge } from '../../components/ModBadges.tsx';
import ModDescription from '../../components/ModDescription.tsx';
import { formatCount } from '../../lib/files.ts';
import { useExtTranslations } from '../../translations.ts';

const PER_PAGE = 15;
const ALL = 'all';
const VISIBLE_GAME_VERSIONS = 3;

export default function ModReleasesTab({
  releases,
  recommendedRelease,
  gameVersion,
  installed,
  canInstall,
  onInstall,
}: {
  releases: Release[];
  recommendedRelease: number | null;
  gameVersion: string | null;
  installed: InstalledMod | null;
  canInstall: boolean;
  onInstall: (release: Release) => void;
}) {
  const { t: tExt } = useExtTranslations();

  // releases list their game versions newest first; keep that order for the filter
  const gameVersions = useMemo(() => {
    const seen = new Set<string>();
    for (const release of releases) for (const version of release.gameVersions) seen.add(version);
    return [...seen];
  }, [releases]);

  const [search, setSearch] = useState('');
  const [filterVersion, setFilterVersion] = useState<string | null>(null);
  const [channel, setChannel] = useState<string>(ALL);
  const [page, setPage] = useState(1);
  const [changelog, setChangelog] = useState<Release | null>(null);

  useEffect(() => {
    if (filterVersion !== null) return;
    setFilterVersion(gameVersion && gameVersions.includes(gameVersion) ? gameVersion : ALL);
  }, [gameVersions, gameVersion]);

  useEffect(() => setPage(1), [search, filterVersion, channel]);

  const needle = search.trim().toLowerCase();
  const filtered = releases.filter(
    (release) =>
      (!filterVersion || filterVersion === ALL || release.gameVersions.includes(filterVersion)) &&
      (channel === ALL || release.prerelease === (channel === 'prerelease')) &&
      (!needle || release.modVersion.toLowerCase().includes(needle) || release.filename.toLowerCase().includes(needle)),
  );
  const pageItems = filtered.slice((page - 1) * PER_PAGE, page * PER_PAGE);

  return (
    <>
      <Modal
        opened={!!changelog}
        onClose={() => setChangelog(null)}
        size='lg'
        title={tExt('pages.server.vsMods.details.releases.changelogTitle', { version: changelog?.modVersion ?? '' })}
      >
        {changelog?.changelog && <ModDescription html={changelog.changelog} />}
      </Modal>

      <Group mb='md' grow align='flex-end'>
        <TextInput
          label={tExt('pages.server.vsMods.details.releases.search', {})}
          placeholder={tExt('pages.server.vsMods.details.releases.searchPlaceholder', {})}
          leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} />}
          value={search}
          onChange={(event) => setSearch(event.currentTarget.value)}
        />
        <Select
          label={tExt('pages.server.vsMods.details.releases.gameVersion', {})}
          data={[{ value: ALL, label: tExt('pages.server.vsMods.gameVersions.any', {}) }, ...gameVersions]}
          value={filterVersion}
          onChange={(value) => setFilterVersion(value ?? ALL)}
          searchable
        />
        <Select
          label={tExt('pages.server.vsMods.details.releases.channel', {})}
          data={[
            { value: ALL, label: tExt('pages.server.vsMods.details.releases.allChannels', {}) },
            { value: 'stable', label: tExt('pages.server.vsMods.channel.stable', {}) },
            { value: 'prerelease', label: tExt('pages.server.vsMods.channel.prerelease', {}) },
          ]}
          value={channel}
          onChange={(value) => setChannel(value ?? ALL)}
        />
      </Group>

      <Table
        columns={[
          tExt('pages.server.vsMods.details.releases.columns.version', {}),
          tExt('pages.server.vsMods.details.releases.columns.channel', {}),
          tExt('pages.server.vsMods.details.releases.columns.gameVersions', {}),
          tExt('pages.server.vsMods.details.releases.columns.compatibility', {}),
          tExt('pages.server.vsMods.details.releases.columns.released', {}),
          tExt('pages.server.vsMods.details.releases.columns.downloads', {}),
          '',
        ]}
        pagination={{ total: filtered.length, perPage: PER_PAGE, page, data: pageItems }}
        onPageSelect={setPage}
        empty={
          <Center py='lg'>
            <Text c='dimmed'>{tExt('pages.server.vsMods.details.releases.empty', {})}</Text>
          </Center>
        }
      >
        {pageItems.map((release) => {
          const isInstalled = installed?.info?.version === release.modVersion;
          const hiddenGameVersions = release.gameVersions.length - VISIBLE_GAME_VERSIONS;

          return (
            <TableRow key={release.releaseId}>
              <TableData>
                <Group gap='xs' wrap='nowrap'>
                  <Code>{release.modVersion}</Code>
                  {release.releaseId === recommendedRelease && (
                    <Tooltip label={tExt('pages.server.vsMods.details.releases.recommended', {})}>
                      <FontAwesomeIcon icon={faStar} color='var(--mantine-color-yellow-5)' />
                    </Tooltip>
                  )}
                  {release.changelog && (
                    <Tooltip label={tExt('pages.server.vsMods.details.releases.changelog', {})}>
                      <ActionIcon variant='subtle' color='gray' size='sm' onClick={() => setChangelog(release)}>
                        <FontAwesomeIcon icon={faFileLines} />
                      </ActionIcon>
                    </Tooltip>
                  )}
                </Group>
                <Text size='xs' c='dimmed' truncate maw={260}>
                  {release.filename}
                </Text>
              </TableData>
              <TableData>
                <ChannelBadge prerelease={release.prerelease} />
              </TableData>
              <TableData>
                <Group gap={4} wrap='nowrap'>
                  <Text size='sm'>{release.gameVersions.slice(0, VISIBLE_GAME_VERSIONS).join(', ')}</Text>
                  {hiddenGameVersions > 0 && (
                    <Tooltip label={release.gameVersions.join(', ')} multiline maw={320}>
                      <Badge variant='default' miw='max-content'>
                        +{hiddenGameVersions}
                      </Badge>
                    </Tooltip>
                  )}
                </Group>
              </TableData>
              <TableData>
                <CompatibilityBadge compatibility={release.compatibility} />
              </TableData>
              <TableData>{release.created && <FormattedTimestamp timestamp={release.created} />}</TableData>
              <TableData>{formatCount(release.downloads)}</TableData>
              <TableData>
                {isInstalled ? (
                  <Badge
                    color='green'
                    variant='light'
                    leftSection={<FontAwesomeIcon icon={faCheck} />}
                    miw='max-content'
                  >
                    {tExt('pages.server.vsMods.details.releases.current', {})}
                  </Badge>
                ) : (
                  canInstall &&
                  release.installable && (
                    <Button
                      size='xs'
                      variant={installed ? 'light' : 'filled'}
                      leftSection={<FontAwesomeIcon icon={installed ? faRightLeft : faDownload} />}
                      onClick={() => onInstall(release)}
                    >
                      {installed
                        ? tExt('pages.server.vsMods.button.changeVersion', {})
                        : tExt('pages.server.vsMods.button.install', {})}
                    </Button>
                  )
                )}
              </TableData>
            </TableRow>
          );
        })}
      </Table>
    </>
  );
}
