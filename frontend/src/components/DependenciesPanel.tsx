import { faCircleCheck, faCircleInfo, faDownload, faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Checkbox from '@/elements/input/Checkbox.tsx';
import Group from '@/elements/layout/Group.tsx';
import Paper from '@/elements/layout/Paper.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import { ModalFooter } from '@/elements/modals/Modal.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import type { DependencyState, DependencyStatus } from '../lib/schemas.ts';
import { getDependencies } from '../api.ts';
import { useInstalledMods } from '../hooks/useInstalledMods.ts';
import { useModEnvironment } from '../hooks/useModEnvironment.ts';
import { modInstallerQueryKey } from '../lib/constants.ts';
import { installRelease } from '../lib/modActions.ts';
import { useExtTranslations } from '../translations.ts';
import { CompatibilityBadge } from './ModBadges.tsx';

/** Dependency states the user should act on. */
const NEEDS_ATTENTION: Record<DependencyState, boolean> = {
  satisfied: false,
  builtin: false,
  game_too_old: true,
  disabled: true,
  outdated: true,
  missing: true,
};

const installable = (dependency: DependencyStatus) => !!dependency.release?.installable;

/**
 * The dependencies of an installed mod (from its `modinfo.json`): missing or outdated ones that
 * the ModDB has a release for can be installed in one go; the rest are listed with the reason.
 * Only the mod's own dependencies are resolved, not theirs.
 */
export default function DependenciesPanel({
  fileName,
  modName,
  onClose,
  closeIfMet = false,
}: {
  fileName: string;
  modName: string;
  onClose: () => void;
  /** Close at once when nothing needs attention (after an install). */
  closeIfMet?: boolean;
}) {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();
  const serverUuid = useServerStore((state) => state.server.uuid);
  const canInstall = useServerCan('files.create');
  const canReplace = useServerCan(['files.create', 'files.delete'], false);
  const { environment } = useModEnvironment();
  const { byModId } = useInstalledMods();

  const { data, error, isLoading } = useQuery({
    queryKey: modInstallerQueryKey('dependencies', serverUuid, fileName),
    queryFn: () => getDependencies(serverUuid, fileName),
    staleTime: 0,
    gcTime: 0,
  });

  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [installing, setInstalling] = useState(false);

  const shown = (data?.dependencies ?? []).filter((dependency) => dependency.state !== 'builtin');
  const needsAttention = shown.filter((dependency) => NEEDS_ATTENTION[dependency.state]);

  useEffect(() => {
    if (!data) return;
    if (closeIfMet && !data.dependencies.some((dependency) => NEEDS_ATTENTION[dependency.state])) {
      onClose();
      return;
    }
    setSelected(new Set(data.dependencies.filter(installable).map((dependency) => dependency.modid)));
  }, [data]);

  const toggle = (modid: string, checked: boolean) => {
    const next = new Set(selected);
    if (checked) next.add(modid);
    else next.delete(modid);
    setSelected(next);
  };

  const doInstall = async () => {
    if (!environment) return;
    setInstalling(true);
    let installed = 0;
    try {
      for (const dependency of shown) {
        if (!selected.has(dependency.modid) || !dependency.release) continue;
        const replacing = dependency.state === 'outdated' ? (byModId.get(dependency.modid) ?? null) : null;
        await installRelease(serverUuid, environment, dependency.release, replacing);
        installed++;
      }
      addToast(tExt('pages.server.vsMods.toast.dependenciesInstalled', { count: installed }), 'success');
      onClose();
    } catch (error) {
      addToast(httpErrorToHuman(error), 'error');
    } finally {
      setInstalling(false);
      queryClient.invalidateQueries({ queryKey: modInstallerQueryKey('mods', serverUuid) });
    }
  };

  if (error) return <Alert color='red'>{httpErrorToHuman(error)}</Alert>;
  if (isLoading || !data) return <Spinner.Centered />;

  const selectedCount = shown.filter((dependency) => selected.has(dependency.modid) && dependency.release).length;
  const needsReplace = shown.some(
    (dependency) => selected.has(dependency.modid) && dependency.state === 'outdated' && dependency.release,
  );

  return (
    <Stack gap='md'>
      <Text>
        {needsAttention.length > 0
          ? tExt('pages.server.vsMods.dependencies.intro', { mod: modName })
          : tExt('pages.server.vsMods.dependencies.allMet', { mod: modName })}
      </Text>

      {shown.length === 0 && (
        <Alert color='gray' icon={<FontAwesomeIcon icon={faCircleInfo} />}>
          {tExt('pages.server.vsMods.dependencies.none', {})}
        </Alert>
      )}

      {shown.map((dependency) => (
        <Paper key={dependency.modid} withBorder p='sm' radius='md'>
          <Group justify='space-between' wrap='nowrap' align='center'>
            <Group gap='sm' wrap='nowrap' miw={0}>
              {installable(dependency) ? (
                <Checkbox
                  checked={selected.has(dependency.modid)}
                  onChange={(event) => toggle(dependency.modid, event.currentTarget.checked)}
                  disabled={installing}
                  aria-label={dependency.modid}
                />
              ) : (
                <FontAwesomeIcon
                  icon={NEEDS_ATTENTION[dependency.state] ? faTriangleExclamation : faCircleCheck}
                  color={
                    NEEDS_ATTENTION[dependency.state] ? 'var(--mantine-color-yellow-5)' : 'var(--mantine-color-green-5)'
                  }
                  fixedWidth
                />
              )}
              <Avatar
                src={dependency.moddb?.logo ?? null}
                name={dependency.moddb?.name ?? dependency.modid}
                size='md'
                radius='md'
              />
              <Stack gap={2} miw={0}>
                <Group gap='xs'>
                  <Text fw={600} truncate>
                    {dependency.moddb?.name ?? dependency.modid}
                  </Text>
                  <Code>{dependency.modid}</Code>
                </Group>
                <Text size='xs' c='dimmed'>
                  {tExt(`pages.server.vsMods.dependencies.state.${dependency.state}`, {
                    version: dependency.requiredVersion || '*',
                    installed: dependency.installedVersion ?? '?',
                  })}
                  {dependency.problem &&
                    ` · ${tExt(`pages.server.vsMods.dependencies.problem.${dependency.problem}`, {
                      version: dependency.requiredVersion || '*',
                    })}`}
                </Text>
              </Stack>
            </Group>
            {dependency.release && (
              <Group gap='xs' wrap='nowrap'>
                <Badge variant='default' miw='max-content'>
                  {dependency.release.modVersion}
                </Badge>
                <CompatibilityBadge compatibility={dependency.release.compatibility} />
              </Group>
            )}
          </Group>
        </Paper>
      ))}

      <ModalFooter>
        {needsAttention.some(installable) && (
          <Button
            leftSection={<FontAwesomeIcon icon={faDownload} />}
            loading={installing}
            disabled={selectedCount === 0 || !canInstall || (needsReplace && !canReplace)}
            onClick={doInstall}
          >
            {tExt('pages.server.vsMods.dependencies.submit', { count: selectedCount })}
          </Button>
        )}
        <Button variant='default' onClick={onClose} disabled={installing}>
          {tExt('pages.server.vsMods.dependencies.close', {})}
        </Button>
      </ModalFooter>
    </Stack>
  );
}
