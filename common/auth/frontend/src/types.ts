import { z } from 'zod';

export const userSchema = z.object({
  id: z.string().uuid(),
  username: z.string().min(3),
  email: z.string().email(),
  created_at: z.string().datetime(),
  updated_at: z.string().datetime()
});

export const loginRequestSchema = z.object({
  username: z.string().min(3),
  password: z.string().min(8)
});

export const registerRequestSchema = z.object({
  username: z.string().min(3),
  email: z.string().email(),
  password: z.string().min(8)
});

export const authResponseSchema = z.object({
  user: userSchema,
  token: z.string()
});

export type User = z.infer<typeof userSchema>;
export type LoginRequest = z.infer<typeof loginRequestSchema>;
export type RegisterRequest = z.infer<typeof registerRequestSchema>;
export type AuthResponse = z.infer<typeof authResponseSchema>; 