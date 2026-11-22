import { faCircleCheck, faCircleQuestion, faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import Badge from '@/elements/data-display/Badge.tsx';
import type { Compatibility, ModSide } from '../lib/schemas.ts';
import { useExtTranslations } from '../translations.ts';

const SIDE_COLORS: Record<ModSide, string> = {
  both: 'blue',
  server: 'grape',
  client: 'orange',
};

/** Where a mod runs: client only, server only, or both. */
export function SideBadge({ side }: { side: ModSide }) {
  const { t: tExt } = useExtTranslations();

  return (
    <Badge variant='light' color={SIDE_COLORS[side]} miw='max-content'>
      {tExt(`pages.server.vsMods.side.${side}`, {})}
    </Badge>
  );
}

/** Stable release or pre-release (`-dev`, `-pre`, `-rc` versions). */
export function ChannelBadge({ prerelease }: { prerelease: boolean }) {
  const { t: tExt } = useExtTranslations();

  return (
    <Badge variant='light' color={prerelease ? 'yellow' : 'green'} miw='max-content'>
      {tExt(prerelease ? 'pages.server.vsMods.channel.prerelease' : 'pages.server.vsMods.channel.stable', {})}
    </Badge>
  );
}

const COMPATIBILITY_STYLE: Record<Compatibility, { color: string; icon: typeof faCircleCheck }> = {
  exact: { color: 'green', icon: faCircleCheck },
  same_major: { color: 'yellow', icon: faCircleCheck },
  incompatible: { color: 'red', icon: faTriangleExclamation },
  unknown: { color: 'gray', icon: faCircleQuestion },
};

/** How a release fits the server's game version. */
export function CompatibilityBadge({ compatibility }: { compatibility: Compatibility }) {
  const { t: tExt } = useExtTranslations();
  const { color, icon } = COMPATIBILITY_STYLE[compatibility];

  return (
    <Badge variant='light' color={color} leftSection={<FontAwesomeIcon icon={icon} />} miw='max-content'>
      {tExt(`pages.server.vsMods.compatibility.${compatibility}`, {})}
    </Badge>
  );
}
