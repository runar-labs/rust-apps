<script lang="ts">
  import { onMount } from 'svelte';
  import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@kagi/ui/card';
  import { Avatar, AvatarFallback, AvatarImage } from '@kagi/ui/avatar';
  import { Button } from '@kagi/ui/button';
  import { profileStore } from '../store';

  let editing = false;

  onMount(() => {
    profileStore.fetchProfile();
  });

  $: profile = $profileStore.profile;
  $: loading = $profileStore.loading;
  $: error = $profileStore.error;
</script>

<Card class="w-[350px]">
  <CardHeader>
    <CardTitle>Profile</CardTitle>
    <CardDescription>View and manage your profile</CardDescription>
  </CardHeader>
  <CardContent>
    {#if loading}
      <div class="flex justify-center">
        <span class="loading loading-spinner loading-md"></span>
      </div>
    {:else if error}
      <p class="text-sm text-red-500">{error}</p>
    {:else if profile}
      <div class="space-y-6">
        <div class="flex items-center space-x-4">
          <Avatar class="h-20 w-20">
            {#if profile.avatar_url}
              <AvatarImage src={profile.avatar_url} alt={profile.display_name} />
            {/if}
            <AvatarFallback>{profile.display_name.slice(0, 2).toUpperCase()}</AvatarFallback>
          </Avatar>
          <div>
            <h3 class="text-lg font-semibold">{profile.display_name}</h3>
            {#if profile.bio}
              <p class="text-sm text-gray-500">{profile.bio}</p>
            {/if}
          </div>
        </div>
        <Button
          variant="outline"
          class="w-full"
          on:click={() => editing = true}
        >
          Edit Profile
        </Button>
      </div>
    {:else}
      <p class="text-sm text-gray-500">No profile found</p>
    {/if}
  </CardContent>
</Card> 