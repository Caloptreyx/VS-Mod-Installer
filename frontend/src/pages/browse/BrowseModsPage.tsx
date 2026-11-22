import { faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { SimpleGrid } from '@mantine/core';
import { useDebouncedValue } from '@mantine/hooks';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import { Pagination } from '@/elements/data-display/Table.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Select from '@/elements/input/Select.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import Paper from '@/elements/layout/Paper.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useServerStore } from '@/stores/server.ts';
import type { InstalledMod, ModSummary } from '../../lib/schemas.ts';
import { searchMods, type SortOption } from '../../api.ts';
import InstallModModal from '../../components/InstallModModal.tsx';
import ModsPageLayout from '../../components/ModsPageLayout.tsx';
import { useInstalledMods } from '../../hooks/useInstalledMods.ts';
import { useModDbFilters } from '../../hooks/useModDbFilters.ts';
import { useModEnvironment } from '../../hooks/useModEnvironment.ts';
import { modInstallerQueryKey, serverModsPath } from '../../lib/constants.ts';
import { useExtTranslations } from '../../translations.ts';
import ModCard from './ModCard.tsx';

const PER_PAGE = 20;
const ANY = 'any';
const SORT_OPTIONS: SortOption[] = ['relevance', 'downloads', 'follows', 'trending', 'updated', 'newest'];

export default function BrowseModsPage() {
  const { t: tExt } = useExtTranslations();
  const navigate = useNavigate();
  const { uuid: serverUuid, uuidShort } = useServerStore((state) => state.server);
  const canInstall = useServerCan('files.create');

  const { environment, loading: environmentLoading } = useModEnvironment();
  const supported = !!environment?.supported;
  const { tags, gameVersions } = useModDbFilters(supported);
  const { byModDbId, byModId } = useInstalledMods({ enabled: supported });

  const [query, setQuery] = useState('');
  const [debouncedQuery] = useDebouncedValue(query, 300);
  const [gameVersion, setGameVersion] = useState<string | null>(null);
  const [tag, setTag] = useState<string>(ANY);
  const [sort, setSort] = useState<SortOption>('relevance');
  const [includeClient, setIncludeClient] = useState(false);
  const [page, setPage] = useState(1);
  const [installTarget, setInstallTarget] = useState<ModSummary | null>(null);

  // Default the game version filter to the server's once it is known.
  useEffect(() => {
    if (environmentLoading || gameVersion !== null) return;
    setGameVersion(environment?.gameVersion ?? ANY);
  }, [environmentLoading]);

  const params = {
    text: debouncedQuery,
    tag: tag === ANY ? null : tag,
    gameVersion: gameVersion === ANY ? null : gameVersion,
    includeClient,
    sort,
    page,
    perPage: PER_PAGE,
  };
  const { data, isFetching, error } = useQuery({
    queryKey: modInstallerQueryKey('search', params),
    queryFn: () => searchMods(serverUuid, params),
    enabled: supported && gameVersion !== null,
    placeholderData: keepPreviousData,
  });

  // Jump back to the first page whenever the result set changes.
  useEffect(() => setPage(1), [debouncedQuery, tag, gameVersion, sort, includeClient]);

  const installedFor = (mod: ModSummary): InstalledMod | null =>
    byModDbId.get(mod.modid) ?? mod.modidstrs.map((id) => byModId.get(id)).find(Boolean) ?? null;

  const gameVersionOptions = [
    ...(environment?.gameVersion && !gameVersions.includes(environment.gameVersion) ? [environment.gameVersion] : []),
    ...gameVersions,
  ];

  const pagination = { total: data?.total ?? 0, perPage: PER_PAGE, page, data: data?.mods ?? [] };

  return (
    <ModsPageLayout>
      {installTarget && (
        <InstallModModal
          opened={!!installTarget}
          onClose={() => setInstallTarget(null)}
          mod={installTarget}
          installed={installedFor(installTarget)}
        />
      )}

      <Paper withBorder p='md' radius='md' mb='md'>
        <Stack gap='sm'>
          <TextInput
            placeholder={tExt('pages.server.vsMods.browse.searchPlaceholder', {})}
            leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} />}
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
          />
          <Group grow align='flex-end'>
            <Select
              label={tExt('pages.server.vsMods.browse.gameVersion', {})}
              data={[{ value: ANY, label: tExt('pages.server.vsMods.gameVersions.any', {}) }, ...gameVersionOptions]}
              value={gameVersion}
              onChange={(value) => setGameVersion(value ?? ANY)}
              searchable
            />
            <Select
              label={tExt('pages.server.vsMods.browse.tag', {})}
              data={[
                { value: ANY, label: tExt('pages.server.vsMods.browse.anyTag', {}) },
                ...tags.map(({ name, count }) => ({ value: name, label: `${name} (${count})` })),
              ]}
              value={tag}
              onChange={(value) => setTag(value ?? ANY)}
              searchable
            />
            <Select
              label={tExt('pages.server.vsMods.browse.sort', {})}
              data={SORT_OPTIONS.map((value) => ({
                value,
                label: tExt(`pages.server.vsMods.browse.sortOptions.${value}`, {}),
              }))}
              value={sort}
              onChange={(value) => setSort((value as SortOption | null) ?? 'relevance')}
            />
          </Group>
          <Switch
            label={tExt('pages.server.vsMods.browse.showClient', {})}
            description={tExt('pages.server.vsMods.browse.showClientDescription', {})}
            checked={includeClient}
            onChange={(event) => setIncludeClient(event.currentTarget.checked)}
          />
        </Stack>
      </Paper>

      {error ? (
        <Alert color='red'>{httpErrorToHuman(error)}</Alert>
      ) : !data ? (
        <Spinner.Centered />
      ) : data.mods.length === 0 ? (
        <EmptyState
          icon={faMagnifyingGlass}
          title={tExt('pages.server.vsMods.browse.empty.title', {})}
          description={tExt('pages.server.vsMods.browse.empty.description', {})}
        />
      ) : (
        <Stack gap='md' style={{ opacity: isFetching ? 0.6 : 1, transition: 'opacity 150ms' }}>
          <Pagination data={pagination} onPageSelect={setPage} />
          <SimpleGrid cols={{ base: 1, xl: 2 }} spacing='md'>
            {data.mods.map((mod) => (
              <ModCard
                key={mod.modid}
                mod={mod}
                installed={installedFor(mod)}
                canInstall={canInstall}
                onOpen={() => navigate(serverModsPath(uuidShort, `/mod/${mod.modid}`))}
                onInstall={() => setInstallTarget(mod)}
              />
            ))}
          </SimpleGrid>
          <Pagination data={pagination} onPageSelect={setPage} />
        </Stack>
      )}
    </ModsPageLayout>
  );
}
