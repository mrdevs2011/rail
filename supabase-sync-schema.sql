-- ============================================================================
-- Rail Browser Cloud Sync — Supabase SQL Migration
-- Run this script in your Supabase Project:
-- Supabase Dashboard -> SQL Editor -> New Query -> Run
-- ============================================================================

-- 1. Create the sync table
CREATE TABLE IF NOT EXISTS public.rail_sync (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    encrypted_data TEXT NOT NULL,
    salt TEXT DEFAULT '',
    iv TEXT DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT timezone('utc'::text, now()),
    expires_at TIMESTAMPTZ DEFAULT (timezone('utc'::text, now()) + interval '30 days')
);

-- 2. Create index for fast lookups by ID
CREATE INDEX IF NOT EXISTS idx_rail_sync_id ON public.rail_sync(id);

-- 3. Enable Row Level Security (RLS)
ALTER TABLE public.rail_sync ENABLE ROW LEVEL SECURITY;

-- 4. Allow public / anonymous insert
DROP POLICY IF EXISTS "Allow public insert for rail_sync" ON public.rail_sync;
CREATE POLICY "Allow public insert for rail_sync"
    ON public.rail_sync
    FOR INSERT
    TO anon, authenticated
    WITH CHECK (true);

-- 5. Allow public / anonymous read by ID
DROP POLICY IF EXISTS "Allow public read for rail_sync" ON public.rail_sync;
CREATE POLICY "Allow public read for rail_sync"
    ON public.rail_sync
    FOR SELECT
    TO anon, authenticated
    USING (true);

-- 6. Verification comment
COMMENT ON TABLE public.rail_sync IS 'Rail Browser end-to-end encrypted zero-knowledge cloud sync storage';
