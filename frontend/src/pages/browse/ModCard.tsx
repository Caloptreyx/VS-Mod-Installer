import { faCheck, faDownload, faHeart, faRightLeft } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import Button from '@/elements/buttons/Button.tsx';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import type { InstalledMod, ModSummary } from '../../lib/schemas.ts';
import { SideBadge } from '../../components/ModBadges.tsx';
import { formatCount } from '../../lib/files.ts';
import { useExtTranslations } from '../../translations.ts';

export default function ModCard({
  mod,
  installed,
  canInstall,
  onOpen,
  onInstall,
}: {
  mod: ModSummary;
  installed: InstalledMod | null;
  canInstall: boolean;
  onOpen: () => void;
  onInstall: () => void;
}) {
  const { t: tExt } = useExtTranslations();

  return (
    <Card hoverable p='md' onClick={onOpen}>
      <Group wrap='nowrap' align='flex-start' gap='md'>
        <Avatar src={mod.logo} name={mod.name} size={64} radius='md' />

        <Stack gap={6} style={{ flex: 1, minWidth: 0 }}>
          <Group justify='space-between' wrap='nowrap' align='flex-start'>
            <div style={{ minWidth: 0 }}>
              <Text fw={600} size='lg' truncate>
                {mod.name}
              </Text>
              <Text size='xs' c='dimmed' truncate>
                {tExt('pages.server.vsMods.browse.by', { author: mod.author })}
              </Text>
            </div>
            {installed && (
              <Badge color='green' variant='light' leftSection={<FontAwesomeIcon icon={faCheck} />} miw='max-content'>
                {tExt('pages.server.vsMods.button.installed', {})}
              </Badge>
            )}
          </Group>

          <Text size='sm' c='dimmed' lineClamp={2}>
            {mod.summary}
          </Text>

          <Group gap={6}>
            <SideBadge side={mod.side} />
            {mod.tags.slice(0, 3).map((tag) => (
              <Badge key={tag} variant='default'>
                {tag}
              </Badge>
            ))}
          </Group>

          <Group justify='space-between' wrap='nowrap' mt={4}>
            <Group gap='md' c='dimmed' wrap='nowrap'>
              <Text
                size='xs'
                title={tExt('pages.server.vsMods.browse.downloads', { count: mod.downloads.toLocaleString() })}
              >
                <FontAwesomeIcon icon={faDownload} /> {formatCount(mod.downloads)}
              </Text>
              <Text
                size='xs'
                title={tExt('pages.server.vsMods.browse.followers', { count: mod.follows.toLocaleString() })}
              >
                <FontAwesomeIcon icon={faHeart} /> {formatCount(mod.follows)}
              </Text>
              {mod.lastReleased && (
                <Text size='xs' visibleFrom='sm'>
                  {tExt('pages.server.vsMods.browse.updated', {})} <FormattedTimestamp timestamp={mod.lastReleased} />
                </Text>
              )}
            </Group>

            {canInstall && (
              <Button
                size='xs'
                variant={installed ? 'light' : 'filled'}
                leftSection={<FontAwesomeIcon icon={installed ? faRightLeft : faDownload} />}
                onClick={(event) => {
                  event.stopPropagation();
                  onInstall();
                }}
              >
                {installed
                  ? tExt('pages.server.vsMods.button.changeVersion', {})
                  : tExt('pages.server.vsMods.button.install', {})}
              </Button>
            )}
          </Group>
        </Stack>
      </Group>
    </Card>
  );
}
