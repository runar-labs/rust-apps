import { z } from 'zod';

export const profileSchema = z.object({
  id: z.string().uuid(),
  user_id: z.string().uuid(),
  display_name: z.string().min(1),
  bio: z.string().nullable(),
  avatar_url: z.string().url().nullable(),
  created_at: z.string().datetime(),
  updated_at: z.string().datetime()
});

export const updateProfileRequestSchema = z.object({
  display_name: z.string().min(1).optional(),
  bio: z.string().optional(),
  avatar_url: z.string().url().optional()
});

export type Profile = z.infer<typeof profileSchema>;
export type UpdateProfileRequest = z.infer<typeof updateProfileRequestSchema>; 