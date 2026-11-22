import { faCircleInfo, faDownload, faRightLeft, faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Select from '@/elements/input/Select.tsx';
import Group from '@/elements/layout/Group.tsx';
import Paper from '@/elements/layout/Paper.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import type { InstalledMod, ModSide, Release } from '../lib/schemas.ts';
import { getModDetail } from '../api.ts';
import { useModEnvironment } from '../hooks/useModEnvironment.ts';
import { modInstallerQueryKey } from '../lib/constants.ts';
import { installRelease } from '../lib/modActions.ts';
import { useExtTranslations } from '../translations.ts';
import DependenciesPanel from './DependenciesPanel.tsx';
import { ChannelBadge, CompatibilityBadge } from './ModBadges.tsx';

export interface InstallTarget {
  /** The ModDB's numeric mod id. */
  modid: number;
  name: string;
  logo: string | null;
  side: ModSide;
}

interface Props {
  opened: boolean;
  onClose: () => void;
  mod: InstallTarget;
  /** The installed file of this mod; installing then replaces it with the selected release. */
  installed: InstalledMod | null;
  /** Release to preselect, e.g. when installing from the Releases tab. */
  initialReleaseId?: number | null;
}

/** `1.22.0 – 1.22.7` for a release's game versions (newest first). */
export const gameVersionRange = (versions: string[]) =>
  versions.length > 1 ? `${versions[versions.length - 1]} – ${versions[0]}` : (versions[0] ?? '?');

export default function InstallModModal({ opened, onClose, mod, installed, initialReleaseId = null }: Props) {
  const { t: tExt } = useExtTranslations();
  const [dependenciesOf, setDependenciesOf] = useState<string | null>(null);

  useEffect(() => {
    if (opened) setDependenciesOf(null);
  }, [opened]);

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      size='lg'
      title={
        <Group gap='sm' wrap='nowrap'>
          <Avatar src={mod.logo} name={mod.name} radius='md' size='md' />
          <Text fw={600}>
            {dependenciesOf
              ? tExt('pages.server.vsMods.dependencies.title', { mod: mod.name })
              : installed
                ? tExt('pages.server.vsMods.installModal.changeTitle', { mod: mod.name })
                : tExt('pages.server.vsMods.installModal.title', { mod: mod.name })}
          </Text>
        </Group>
      }
    >
      {opened &&
        (dependenciesOf ? (
          <DependenciesPanel fileName={dependenciesOf} modName={mod.name} onClose={onClose} closeIfMet />
        ) : (
          <InstallForm
            mod={mod}
            installed={installed}
            initialReleaseId={initialReleaseId}
            onClose={onClose}
            onInstalled={setDependenciesOf}
          />
        ))}
    </Modal>
  );
}

function InstallForm({
  mod,
  installed,
  initialReleaseId,
  onClose,
  onInstalled,
}: Omit<Props, 'opened' | 'initialReleaseId'> & {
  initialReleaseId: number | null;
  onInstalled: (fileName: string) => void;
}) {
  const { t } = useTranslations();
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();
  const serverUuid = useServerStore((state) => state.server.uuid);
  const canDelete = useServerCan('files.delete');
  const { environment, loading: environmentLoading } = useModEnvironment();
  const gameVersion = environment?.gameVersion ?? null;

  const detail = useQuery({
    queryKey: modInstallerQueryKey('mod', String(mod.modid), gameVersion),
    queryFn: () => getModDetail(serverUuid, mod.modid, gameVersion),
    enabled: !environmentLoading,
  });

  const [releaseId, setReleaseId] = useState<number | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!detail.data || releaseId !== null) return;
    setReleaseId(initialReleaseId ?? detail.data.recommendedRelease);
  }, [detail.data]);

  if (detail.error) return <Alert color='red'>{httpErrorToHuman(detail.error)}</Alert>;
  if (environmentLoading || !detail.data || !environment) return <Spinner.Centered />;

  const releases = detail.data.releases.filter((release) => release.installable);
  const selected: Release | undefined = releases.find((release) => release.releaseId === releaseId);
  const alreadyInstalled = !!selected && installed?.info?.version === selected.modVersion;
  const replacing = !!installed && !alreadyInstalled;

  const doInstall = () => {
    if (!selected) return;

    setSubmitting(true);
    installRelease(serverUuid, environment, selected, installed)
      .then((fileName) => {
        const values = { mod: mod.name, version: selected.modVersion };
        addToast(
          installed
            ? tExt('pages.server.vsMods.toast.changed', values)
            : tExt('pages.server.vsMods.toast.installed', values),
          'success',
        );
        queryClient.invalidateQueries({ queryKey: modInstallerQueryKey('mods', serverUuid) });
        queryClient.invalidateQueries({ queryKey: modInstallerQueryKey('environment', serverUuid) });
        onInstalled(fileName);
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setSubmitting(false));
  };

  return (
    <Stack gap='md'>
      <Select
        label={tExt('pages.server.vsMods.installModal.release', {})}
        description={
          gameVersion ? tExt('pages.server.vsMods.installModal.recommendedHint', { version: gameVersion }) : undefined
        }
        data={releases.map((release) => ({
          value: String(release.releaseId),
          label: `${release.modVersion}${release.releaseId === detail.data.recommendedRelease ? ' ★' : ''} · ${tExt(
            'pages.server.vsMods.installModal.forGame',
            { versions: gameVersionRange(release.gameVersions) },
          )}`,
        }))}
        value={selected ? String(selected.releaseId) : null}
        onChange={(value) => setReleaseId(value ? Number(value) : null)}
        searchable
        disabled={releases.length === 0}
      />

      {releases.length === 0 && (
        <Alert color='gray' icon={<FontAwesomeIcon icon={faCircleInfo} />}>
          {tExt('pages.server.vsMods.installModal.noReleases', {})}
        </Alert>
      )}

      {selected && (
        <Paper withBorder p='sm' radius='md'>
          <Group justify='space-between' wrap='nowrap'>
            <Stack gap={4} miw={0}>
              <Group gap='xs'>
                <Text fw={600}>{selected.modVersion}</Text>
                <ChannelBadge prerelease={selected.prerelease} />
                <CompatibilityBadge compatibility={selected.compatibility} />
              </Group>
              <Text size='sm' c='dimmed' truncate>
                {tExt('pages.server.vsMods.installModal.file', {})}: <Code>{selected.filename}</Code>
              </Text>
              <Text size='sm' c='dimmed'>
                {tExt('pages.server.vsMods.installModal.forGame', {
                  versions: gameVersionRange(selected.gameVersions),
                })}
              </Text>
            </Stack>
            {selected.created && (
              <Text size='xs' c='dimmed'>
                <FormattedTimestamp timestamp={selected.created} />
              </Text>
            )}
          </Group>
        </Paper>
      )}

      {alreadyInstalled && (
        <Alert color='blue' icon={<FontAwesomeIcon icon={faCircleInfo} />}>
          {tExt('pages.server.vsMods.installModal.alreadyInstalled', {})}
        </Alert>
      )}

      {installed && replacing && (
        <Alert color='blue' icon={<FontAwesomeIcon icon={faRightLeft} />}>
          {tExt('pages.server.vsMods.installModal.currentVersion', {
            version: installed.info?.version ?? installed.fileName,
          }).md()}
        </Alert>
      )}

      {selected && gameVersion && selected.compatibility === 'incompatible' && (
        <Alert color='yellow' icon={<FontAwesomeIcon icon={faTriangleExclamation} />}>
          {tExt('pages.server.vsMods.installModal.incompatible', {
            versions: gameVersionRange(selected.gameVersions),
            detected: gameVersion,
          })}
        </Alert>
      )}

      {selected && gameVersion && selected.compatibility === 'same_major' && (
        <Alert color='blue' icon={<FontAwesomeIcon icon={faCircleInfo} />}>
          {tExt('pages.server.vsMods.installModal.sameMajor', {
            versions: gameVersionRange(selected.gameVersions),
            detected: gameVersion,
          })}
        </Alert>
      )}

      {!gameVersion && (
        <Alert color='yellow' icon={<FontAwesomeIcon icon={faTriangleExclamation} />}>
          {tExt('pages.server.vsMods.installModal.notDetected', {})}
        </Alert>
      )}

      {mod.side === 'client' && (
        <Alert color='yellow' icon={<FontAwesomeIcon icon={faTriangleExclamation} />}>
          {tExt('pages.server.vsMods.installModal.clientOnly', {})}
        </Alert>
      )}

      <ModalFooter>
        <Button
          leftSection={<FontAwesomeIcon icon={installed ? faRightLeft : faDownload} />}
          loading={submitting}
          disabled={!selected || alreadyInstalled || (replacing && !canDelete)}
          onClick={doInstall}
        >
          {installed
            ? tExt('pages.server.vsMods.installModal.submitChange', {})
            : tExt('pages.server.vsMods.installModal.submit', {})}
        </Button>
        <Button variant='default' onClick={onClose}>
          {t('common.button.cancel', {})}
        </Button>
      </ModalFooter>
    </Stack>
  );
}
