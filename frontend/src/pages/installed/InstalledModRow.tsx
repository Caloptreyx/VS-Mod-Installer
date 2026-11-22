import {
  faArrowUpRightFromSquare,
  faCircleUp,
  faDiagramProject,
  faRightLeft,
  faToggleOff,
  faToggleOn,
  faTrash,
  faTriangleExclamation,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { UnstyledButton } from '@mantine/core';
import { useState } from 'react';
import { useNavigate } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import Switch from '@/elements/input/Switch.tsx';
import Group from '@/elements/layout/Group.tsx';
import ConfirmationModal from '@/elements/modals/ConfirmationModal.tsx';
import { Modal } from '@/elements/modals/Modal.tsx';
import ContextMenu, { ContextMenuToggle } from '@/elements/overlays/ContextMenu.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { bytesToString } from '@/lib/format/size.ts';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import type { InstalledMod, ModUpdate } from '../../lib/schemas.ts';
import DependenciesPanel from '../../components/DependenciesPanel.tsx';
import InstallModModal from '../../components/InstallModModal.tsx';
import { serverModsPath } from '../../lib/constants.ts';
import { deleteMod, setModEnabled } from '../../lib/modActions.ts';
import { useExtTranslations } from '../../translations.ts';

/** Display name of a mod: its modinfo name, else the file name without extensions. */
export const modDisplayName = (mod: InstalledMod) =>
  mod.info?.name ?? mod.fileName.replace(/\.(zip|dll|cs)(\.disabled)?$/i, '');

export default function InstalledModRow({
  mod,
  modsDir,
  update,
  onChanged,
}: {
  mod: InstalledMod;
  modsDir: string;
  update: ModUpdate | null;
  onChanged: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const navigate = useNavigate();
  const server = useServerStore((state) => state.server);

  const canUpdate = useServerCan('files.update');
  const canChange = useServerCan(['files.create', 'files.delete'], false);
  const canDelete = useServerCan('files.delete');

  const [openModal, setOpenModal] = useState<'install' | 'update' | 'dependencies' | 'delete' | null>(null);
  const [toggling, setToggling] = useState(false);

  const name = modDisplayName(mod);
  const canToggle = mod.kind !== 'folder';

  const doToggle = (enabled: boolean) => {
    setToggling(true);
    setModEnabled(server.uuid, modsDir, mod, enabled)
      .then(() => {
        addToast(
          enabled
            ? tExt('pages.server.vsMods.toast.enabled', { mod: name })
            : tExt('pages.server.vsMods.toast.disabled', { mod: name }),
          'success',
        );
        onChanged();
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setToggling(false));
  };

  const doDelete = async () => {
    await deleteMod(server.uuid, modsDir, mod);
    addToast(tExt('pages.server.vsMods.toast.deleted', { mod: name }), 'success');
    setOpenModal(null);
    onChanged();
  };

  return (
    <>
      {mod.moddb && (
        <InstallModModal
          opened={openModal === 'install' || openModal === 'update'}
          onClose={() => setOpenModal(null)}
          mod={{ modid: mod.moddb.modid, name, logo: mod.moddb.logo, side: mod.moddb.side }}
          installed={mod}
          initialReleaseId={openModal === 'update' ? (update?.release.releaseId ?? null) : null}
        />
      )}
      <Modal
        opened={openModal === 'dependencies'}
        onClose={() => setOpenModal(null)}
        size='lg'
        title={tExt('pages.server.vsMods.dependencies.title', { mod: name })}
      >
        {openModal === 'dependencies' && (
          <DependenciesPanel fileName={mod.fileName} modName={name} onClose={() => setOpenModal(null)} />
        )}
      </Modal>
      <ConfirmationModal
        opened={openModal === 'delete'}
        onClose={() => setOpenModal(null)}
        title={tExt('pages.server.vsMods.installed.modal.delete.title', {})}
        confirm={tExt('pages.server.vsMods.button.delete', {})}
        onConfirmed={doDelete}
      >
        {tExt('pages.server.vsMods.installed.modal.delete.content', { mod: name, file: mod.fileName }).md()}
      </ConfirmationModal>

      <ContextMenu
        items={[
          {
            type: 'action',
            icon: faArrowUpRightFromSquare,
            label: tExt('pages.server.vsMods.button.viewDetails', {}),
            onClick: () => {
              if (mod.moddb) navigate(serverModsPath(server.uuidShort, `/mod/${mod.moddb.modid}`));
            },
            color: 'gray',
            hidden: !mod.moddb,
          },
          {
            type: 'action',
            icon: faCircleUp,
            label: tExt('pages.server.vsMods.button.update', {}),
            onClick: () => setOpenModal('update'),
            color: 'gray',
            hidden: !update || !mod.moddb,
            canAccess: canChange,
          },
          {
            type: 'action',
            icon: faRightLeft,
            label: tExt('pages.server.vsMods.button.changeVersion', {}),
            onClick: () => setOpenModal('install'),
            color: 'gray',
            hidden: !mod.moddb,
            canAccess: canChange,
          },
          {
            type: 'action',
            icon: faDiagramProject,
            label: tExt('pages.server.vsMods.button.dependencies', {}),
            onClick: () => setOpenModal('dependencies'),
            color: 'gray',
            hidden: !mod.info || mod.info.dependencies.length === 0,
          },
          {
            type: 'action',
            icon: mod.enabled ? faToggleOff : faToggleOn,
            label: mod.enabled
              ? tExt('pages.server.vsMods.button.disable', {})
              : tExt('pages.server.vsMods.button.enable', {}),
            onClick: () => doToggle(!mod.enabled),
            color: 'gray',
            hidden: !canToggle,
            canAccess: canUpdate,
          },
          {
            type: 'action',
            icon: faTrash,
            label: tExt('pages.server.vsMods.button.delete', {}),
            onClick: () => setOpenModal('delete'),
            color: 'red',
            canAccess: canDelete,
          },
        ]}
      >
        {({ items, openMenu }) => (
          <TableRow
            style={{ opacity: mod.enabled ? 1 : 0.6 }}
            onContextMenu={(event) => {
              event.preventDefault();
              openMenu(event.clientX, event.clientY);
            }}
          >
            <TableData>
              <Group gap='sm' wrap='nowrap'>
                <Avatar src={mod.moddb?.logo ?? null} name={name} radius='md' size='md' />
                <div style={{ minWidth: 0 }}>
                  <Text fw={600} truncate maw={320}>
                    {name}
                  </Text>
                  {mod.unmetDependencies.length > 0 ? (
                    <Text
                      size='xs'
                      c='yellow'
                      truncate
                      maw={320}
                      style={{ cursor: 'pointer' }}
                      onClick={() => setOpenModal('dependencies')}
                    >
                      <FontAwesomeIcon icon={faTriangleExclamation} />{' '}
                      {tExt('pages.server.vsMods.installed.unmetDependencies', {
                        mods: mod.unmetDependencies.join(', '),
                      })}
                    </Text>
                  ) : mod.infoError ? (
                    <Text size='xs' c='dimmed' truncate maw={320}>
                      {tExt(`pages.server.vsMods.installed.infoError.${mod.infoError}`, {})}
                    </Text>
                  ) : (
                    <Text size='xs' c='dimmed' truncate maw={320}>
                      {mod.moddb?.summary ??
                        mod.info?.description ??
                        tExt('pages.server.vsMods.installed.notOnModDb', {})}
                    </Text>
                  )}
                </div>
              </Group>
            </TableData>

            <TableData>
              <Group gap='xs' wrap='nowrap'>
                {mod.info?.version ? (
                  <Code>{mod.info.version}</Code>
                ) : (
                  <Text size='sm' c='dimmed'>
                    {tExt('pages.server.vsMods.installed.unknownVersion', {})}
                  </Text>
                )}
                {update && (
                  <Tooltip label={tExt('pages.server.vsMods.button.update', {})}>
                    <UnstyledButton disabled={!canChange} onClick={() => setOpenModal('update')}>
                      <Badge
                        color='blue'
                        variant='light'
                        leftSection={<FontAwesomeIcon icon={faCircleUp} />}
                        style={{ cursor: canChange ? 'pointer' : undefined }}
                        miw='max-content'
                      >
                        {update.release.modVersion}
                      </Badge>
                    </UnstyledButton>
                  </Tooltip>
                )}
              </Group>
            </TableData>

            <TableData>
              <Text size='sm' c='dimmed' truncate maw={260}>
                {mod.fileName}
              </Text>
            </TableData>

            <TableData>{mod.kind === 'folder' ? '-' : bytesToString(mod.size)}</TableData>

            <TableData>
              {canToggle && (
                <Switch
                  checked={mod.enabled}
                  disabled={!canUpdate || toggling}
                  onChange={(event) => doToggle(event.currentTarget.checked)}
                  aria-label={
                    mod.enabled
                      ? tExt('pages.server.vsMods.button.disable', {})
                      : tExt('pages.server.vsMods.button.enable', {})
                  }
                />
              )}
            </TableData>

            <ContextMenuToggle items={items} openMenu={openMenu} />
          </TableRow>
        )}
      </ContextMenu>
    </>
  );
}
