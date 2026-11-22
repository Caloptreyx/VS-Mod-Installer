import { faBoxArchive, faServer } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import getAllEggs from '@/api/admin/nests/getAllEggs.ts';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import TitleCard from '@/elements/data-display/TitleCard.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import MultiSelect from '@/elements/input/MultiSelect.tsx';
import NumberInput from '@/elements/input/NumberInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { getSettings, type Settings, updateSettings } from '../../api.ts';
import { modInstallerQueryKey } from '../../lib/constants.ts';
import { useExtTranslations } from '../../translations.ts';

const settingsKey = modInstallerQueryKey('settings');

export default function AdminConfigurationPage() {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();
  const canRead = useAdminCan('extensions.read');
  const canManage = useAdminCan('extensions.manage');
  const query = useQuery({ queryKey: settingsKey, queryFn: getSettings, enabled: canRead });
  const eggs = useQuery({ queryKey: modInstallerQueryKey('eggs'), queryFn: getAllEggs, enabled: canRead });
  const [form, setForm] = useState<Settings | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (query.data) setForm(query.data);
  }, [query.data]);

  if (query.isLoading) {
    return (
      <Group justify='center' py='lg'>
        <Spinner />
      </Group>
    );
  }
  if (query.error || !form) {
    return <Alert color='red'>{query.error ? httpErrorToHuman(query.error) : null}</Alert>;
  }

  const save = async () => {
    setSaving(true);
    try {
      const saved = await updateSettings(form);
      setForm(saved);
      queryClient.setQueryData(settingsKey, saved);
      addToast(tExt('admin.saved', {}), 'success');
    } catch (error) {
      addToast(httpErrorToHuman(error), 'error');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Stack>
      <TitleCard title={tExt('admin.access', {})} icon={<FontAwesomeIcon icon={faServer} />}>
        <MultiSelect
          label={tExt('admin.eggs', {})}
          description={tExt('admin.eggsDescription', {})}
          searchable
          data={(eggs.data ?? []).map(({ nest, eggs: nestEggs }) => ({
            group: nest.name,
            items: nestEggs.map((egg) => ({ value: egg.uuid, label: egg.name })),
          }))}
          value={form.eggs}
          onChange={(value) => setForm({ ...form, eggs: value })}
          disabled={!canManage}
        />
      </TitleCard>
      <TitleCard title={tExt('admin.inspection', {})} icon={<FontAwesomeIcon icon={faBoxArchive} />}>
        <NumberInput
          label={tExt('admin.maxArchive', {})}
          description={tExt('admin.maxArchiveDescription', {})}
          min={1}
          max={1024}
          allowDecimal={false}
          value={form.max_archive_mb}
          onChange={(value) => typeof value === 'number' && setForm({ ...form, max_archive_mb: value })}
          disabled={!canManage}
        />
      </TitleCard>
      {canManage && (
        <Group justify='flex-end'>
          <Button loading={saving} onClick={save}>
            {tExt('admin.save', {})}
          </Button>
        </Group>
      )}
    </Stack>
  );
}
