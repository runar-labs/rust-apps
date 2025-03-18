<script lang="ts">
  import { Button } from '@kagi/ui/button';
  import { Input } from '@kagi/ui/input';
  import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@kagi/ui/card';
  import { Form } from '@kagi/ui/form';
  import { loginRequestSchema } from '../types';
  import { createForm } from '@kagi/ui/form';
  import { authStore } from '../store';

  const form = createForm(loginRequestSchema);

  let error = '';
  let loading = false;

  async function onSubmit(data: FormData) {
    loading = true;
    error = '';

    try {
      const response = await fetch('/api/auth/login', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
      });

      if (!response.ok) {
        throw new Error('Invalid username or password');
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
    <CardTitle>Login</CardTitle>
    <CardDescription>Enter your credentials to access your account</CardDescription>
  </CardHeader>
  <CardContent>
    <Form {form} {onSubmit}>
      <div class="space-y-4">
        <Input
          name="username"
          label="Username"
          placeholder="Enter your username"
          required
        />
        <Input
          name="password"
          type="password"
          label="Password"
          placeholder="Enter your password"
          required
        />
        {#if error}
          <p class="text-sm text-red-500">{error}</p>
        {/if}
      </div>
    </Form>
  </CardContent>
  <CardFooter class="flex justify-between">
    <Button variant="outline" href="/register">Create Account</Button>
    <Button type="submit" disabled={loading}>
      {loading ? 'Logging in...' : 'Login'}
    </Button>
  </CardFooter>
</Card> 