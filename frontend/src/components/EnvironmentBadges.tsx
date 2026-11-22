import { faCube, faFolderOpen } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Skeleton } from '@mantine/core';
import Badge from '@/elements/data-display/Badge.tsx';
import Group from '@/elements/layout/Group.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import type { DetectionSource, ServerEnvironment } from '../lib/schemas.ts';
import { useExtTranslations } from '../translations.ts';

/** The detected game version and mods folder; hover a badge to see where it was found. */
export default function EnvironmentBadges({
  environment,
  loading,
}: {
  environment: ServerEnvironment | undefined;
  loading: boolean;
}) {
  const { t: tExt } = useExtTranslations();

  if (loading) {
    return (
      <Group gap='xs'>
        <Skeleton height={22} width={130} radius='xl' />
        <Skeleton height={22} width={110} radius='xl' />
      </Group>
    );
  }

  if (!environment?.supported) return null;

  const sourceLabel = (source: DetectionSource | null) =>
    source
      ? tExt('pages.server.vsMods.environment.detectedFrom', {
          source: tExt(`pages.server.vsMods.environment.source.${source}`, {}),
        })
      : tExt('pages.server.vsMods.environment.notDetected', {});

  return (
    <Group gap='xs'>
      <Tooltip label={sourceLabel(environment.gameVersionSource)}>
        <Badge
          size='lg'
          variant='light'
          color={environment.gameVersion ? 'green' : 'gray'}
          leftSection={<FontAwesomeIcon icon={faCube} />}
        >
          {environment.gameVersion
            ? tExt('pages.server.vsMods.environment.gameVersion', { version: environment.gameVersion })
            : tExt('pages.server.vsMods.environment.unknownVersion', {})}
        </Badge>
      </Tooltip>
      <Tooltip label={sourceLabel(environment.modsDirSource)}>
        <Badge size='lg' variant='light' color='blue' leftSection={<FontAwesomeIcon icon={faFolderOpen} />}>
          {environment.modsDir}
        </Badge>
      </Tooltip>
    </Group>
  );
}
