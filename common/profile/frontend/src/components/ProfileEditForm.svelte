<script lang="ts">
  import { Button } from '@kagi/ui/button';
  import { Input } from '@kagi/ui/input';
  import { Textarea } from '@kagi/ui/textarea';
  import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@kagi/ui/card';
  import { Form } from '@kagi/ui/form';
  import { updateProfileRequestSchema } from '../types';
  import { createForm } from '@kagi/ui/form';
  import { profileStore } from '../store';

  export let onCancel: () => void;

  const form = createForm(updateProfileRequestSchema, {
    defaultValues: {
      display_name: $profileStore.profile?.display_name,
      bio: $profileStore.profile?.bio || '',
      avatar_url: $profileStore.profile?.avatar_url || ''
    }
  });

  let error = '';
  let loading = false;

  async function onSubmit(data: FormData) {
    loading = true;
    error = '';

    try {
      await profileStore.updateProfile(data);
      onCancel();
    } catch (e) {
      error = e instanceof Error ? e.message : 'An error occurred';
    } finally {
      loading = false;
    }
  }
</script>

<Card class="w-[350px]">
  <CardHeader>
    <CardTitle>Edit Profile</CardTitle>
    <CardDescription>Update your profile information</CardDescription>
  </CardHeader>
  <CardContent>
    <Form {form} {onSubmit}>
      <div class="space-y-4">
        <Input
          name="display_name"
          label="Display Name"
          placeholder="Enter your display name"
          required
        />
        <Input
          name="avatar_url"
          label="Avatar URL"
          placeholder="Enter avatar image URL"
        />
        <Textarea
          name="bio"
          label="Bio"
          placeholder="Tell us about yourself"
          rows={4}
        />
        {#if error}
          <p class="text-sm text-red-500">{error}</p>
        {/if}
      </div>
    </Form>
  </CardContent>
  <CardFooter class="flex justify-between">
    <Button variant="outline" on:click={onCancel}>Cancel</Button>
    <Button type="submit" disabled={loading}>
      {loading ? 'Saving...' : 'Save Changes'}
    </Button>
  </CardFooter>
</Card> 