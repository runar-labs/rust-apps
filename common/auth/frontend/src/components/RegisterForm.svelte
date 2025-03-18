<script lang="ts">
  import { Button } from '@kagi/ui/button';
  import { Input } from '@kagi/ui/input';
  import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@kagi/ui/card';
  import { Form } from '@kagi/ui/form';
  import { registerRequestSchema } from '../types';
  import { createForm } from '@kagi/ui/form';
  import { authStore } from '../store';

  const form = createForm(registerRequestSchema);

  let error = '';
  let loading = false;

  async function onSubmit(data: FormData) {
    loading = true;
    error = '';

    try {
      const response = await fetch('/api/auth/register', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
      });

      if (!response.ok) {
        const result = await response.json();
        throw new Error(result.message || 'Registration failed');
      }

      const result = await response.json();
      authStore.setAuth(result.user, result.token);
    } catch (e) {
      error = e instanceof Error ? e.message : 'An error occurred';
    } finally {
      loading = false;
    }
  }
</script>

<Card class="w-[350px]">
  <CardHeader>
    <CardTitle>Create Account</CardTitle>
    <CardDescription>Enter your details to create a new account</CardDescription>
  </CardHeader>
  <CardContent>
    <Form {form} {onSubmit}>
      <div class="space-y-4">
        <Input
          name="username"
          label="Username"
          placeholder="Choose a username"
          required
        />
        <Input
          name="email"
          type="email"
          label="Email"
          placeholder="Enter your email"
          required
        />
        <Input
          name="password"
          type="password"
          label="Password"
          placeholder="Choose a password"
          required
        />
        {#if error}
          <p class="text-sm text-red-500">{error}</p>
        {/if}
      </div>
    </Form>
  </CardContent>
  <CardFooter class="flex justify-between">
    <Button variant="outline" href="/login">Back to Login</Button>
    <Button type="submit" disabled={loading}>
      {loading ? 'Creating...' : 'Create Account'}
    </Button>
  </CardFooter>
</Card> 