import { faArrowsRotate, faCompass, faCubes, faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center } from '@mantine/core';
import { useState } from 'react';
import { useNavigate } from 'react-router';
import Button from '@/elements/buttons/Button.tsx';
import Table from '@/elements/data-display/Table.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerStore } from '@/stores/server.ts';
import ModsPageLayout from '../../components/ModsPageLayout.tsx';
import { useInstalledMods } from '../../hooks/useInstalledMods.ts';
import { useModEnvironment } from '../../hooks/useModEnvironment.ts';
import { serverModsPath } from '../../lib/constants.ts';
import { useExtTranslations } from '../../translations.ts';
import InstalledModRow, { modDisplayName } from './InstalledModRow.tsx';

type StatusFilter = 'all' | 'enabled' | 'disabled' | 'updates';

export default function InstalledModsPage() {
  const { t: tExt } = useExtTranslations();
  const navigate = useNavigate();
  const uuidShort = useServerStore((state) => state.server.uuidShort);

  const { environment } = useModEnvironment();
  const {
    mods,
    modsDir,
    catalogError,
    updatesByFile,
    loading,
    error,
    refreshing,
    checkingUpdates,
    updatesError,
    refresh,
  } = useInstalledMods({ enabled: !!environment?.supported, withUpdates: true });

  const [search, setSearch] = useState('');
  const [status, setStatus] = useState<StatusFilter>('all');

  const enabledCount = mods?.filter((mod) => mod.enabled).length ?? 0;
  const needle = search.trim().toLowerCase();
  const visibleMods = (mods ?? []).filter(
    (mod) =>
      (status === 'all' ||
        (status === 'updates' ? updatesByFile.has(mod.fileName) : mod.enabled === (status === 'enabled'))) &&
      (!needle ||
        modDisplayName(mod).toLowerCase().includes(needle) ||
        mod.fileName.toLowerCase().includes(needle) ||
        !!mod.info?.modid.includes(needle)),
  );

  const browseButton = (
    <Button
      leftSection={<FontAwesomeIcon icon={faCompass} />}
      onClick={() => navigate(serverModsPath(uuidShort, '/browse'))}
    >
      {tExt('pages.server.vsMods.button.browse', {})}
    </Button>
  );

  return (
    <ModsPageLayout>
      {catalogError && (
        <Alert color='yellow' mb='md'>
          {tExt('pages.server.vsMods.installed.catalogFailed', { error: catalogError })}
        </Alert>
      )}
      {updatesError && (
        <Alert color='yellow' mb='md'>
          {tExt('pages.server.vsMods.installed.updatesFailed', { error: updatesError })}
        </Alert>
      )}

      {loading ? (
        <Spinner.Centered />
      ) : mods && mods.length === 0 && !error ? (
        <EmptyState
          icon={faCubes}
          title={tExt('pages.server.vsMods.installed.empty.title', {})}
          description={tExt('pages.server.vsMods.installed.empty.description', {
            path: modsDir ?? environment?.modsDir ?? '',
          })}
        >
          {browseButton}
        </EmptyState>
      ) : (
        <>
          <Group justify='space-between' mb='md'>
            <Group gap='sm'>
              <TextInput
                placeholder={tExt('pages.server.vsMods.installed.searchPlaceholder', {})}
                leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} />}
                value={search}
                onChange={(event) => setSearch(event.currentTarget.value)}
                w={260}
              />
              <SegmentedControl
                value={status}
                onChange={(value) => setStatus(value as StatusFilter)}
                data={(['all', 'enabled', 'disabled', 'updates'] as const).map((value) => ({
                  value,
                  label:
                    value === 'updates'
                      ? tExt('pages.server.vsMods.installed.filter.updates', { count: updatesByFile.size })
                      : tExt(`pages.server.vsMods.installed.filter.${value}`, {}),
                }))}
              />
            </Group>
            <Group gap='sm'>
              {mods && (
                <Text size='sm' c='dimmed'>
                  {tExt('pages.server.vsMods.installed.summary', {
                    total: mods.length,
                    enabled: enabledCount,
                    disabled: mods.length - enabledCount,
                  })}
                </Text>
              )}
              <Button
                variant='default'
                leftSection={<FontAwesomeIcon icon={faArrowsRotate} spin={refreshing} />}
                onClick={refresh}
              >
                {tExt('pages.server.vsMods.button.refresh', {})}
              </Button>
              {browseButton}
            </Group>
          </Group>

          {checkingUpdates && (
            <Text size='sm' c='dimmed' mb='xs'>
              {tExt('pages.server.vsMods.installed.checkingUpdates', {})}
            </Text>
          )}

          <Table
            columns={[
              tExt('pages.server.vsMods.installed.columns.mod', {}),
              tExt('pages.server.vsMods.installed.columns.version', {}),
              tExt('pages.server.vsMods.installed.columns.file', {}),
              tExt('pages.server.vsMods.installed.columns.size', {}),
              tExt('pages.server.vsMods.installed.columns.enabled', {}),
              '',
            ]}
            error={error}
            pagination={{
              total: visibleMods.length,
              perPage: Math.max(visibleMods.length, 1),
              page: 1,
              data: visibleMods,
            }}
            empty={
              <Center py='lg'>
                <Text c='dimmed'>{tExt('pages.server.vsMods.installed.noMatches', {})}</Text>
              </Center>
            }
          >
            {visibleMods.map((mod) => (
              <InstalledModRow
                key={mod.fileName}
                mod={mod}
                modsDir={modsDir ?? environment?.modsDir ?? ''}
                update={updatesByFile.get(mod.fileName) ?? null}
                onChanged={refresh}
              />
            ))}
          </Table>
        </>
      )}
    </ModsPageLayout>
  );
}
