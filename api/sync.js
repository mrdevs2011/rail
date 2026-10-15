import { createClient } from '@supabase/supabase-js';

// Fallback to fetch if supabase-js not installed or for zero cold start
async function querySupabase(url, anonKey, method, path, body = null) {
  const fullUrl = `${url.replace(/\/$/, '')}/rest/v1/${path}`;
  const headers = {
    'apikey': anonKey,
    'Authorization': `Bearer ${anonKey}`,
    'Content-Type': 'application/json',
    'Accept': 'application/json'
  };
  if (method === 'POST') {
    headers['Prefer'] = 'return=representation';
  }

  const res = await fetch(fullUrl, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined
  });

  if (!res.ok) {
    const errText = await res.text();
    throw new Error(`Supabase error (${res.status}): ${errText}`);
  }
  return res.json();
}

export default async function handler(req, res) {
  // CORS headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization, apikey');
  res.setHeader('Cache-Control', 'no-store');

  if (req.method === 'OPTIONS') {
    return res.status(204).end();
  }

  const supabaseUrl = process.env.SUPABASE_URL;
  const supabaseAnon = process.env.SUPABASE_ANON;

  // Handle configuration status check
  if (req.method === 'GET' && req.query.action === 'config') {
    return res.status(200).json({
      configured: Boolean(supabaseUrl && supabaseAnon),
      supabase_url: supabaseUrl || null
    });
  }

  if (!supabaseUrl || !supabaseAnon) {
    return res.status(500).json({
      success: false,
      error: 'SUPABASE_URL and SUPABASE_ANON environment variables are not configured in Vercel. Please add them in Vercel Project Settings.'
    });
  }

  try {
    if (req.method === 'POST') {
      let body = req.body;
      if (typeof body === 'string') {
        try {
          body = JSON.parse(body);
        } catch {
          return res.status(400).json({ success: false, error: 'Invalid JSON payload' });
        }
      }

      const encryptedData = body?.encrypted_data;
      if (!encryptedData || typeof encryptedData !== 'string' || encryptedData.length < 10) {
        return res.status(400).json({
          success: false,
          error: 'Missing or invalid encrypted_data payload.'
        });
      }

      // Generate a clean UUID v4
      const id = (globalThis.crypto && crypto.randomUUID) ? crypto.randomUUID() : (
        '10000000-1000-4000-8000-100000000000'.replace(/[018]/g, c =>
          (c ^ (crypto.getRandomValues(new Uint8Array(1))[0] & (15 >> (c / 4)))).toString(16)
        )
      );

      const record = {
        id,
        encrypted_data: encryptedData,
        salt: body.salt || '',
        iv: body.iv || ''
      };

      await querySupabase(supabaseUrl, supabaseAnon, 'POST', 'rail_sync', record);

      const host = req.headers['x-forwarded-host'] || req.headers.host || 'rail-browser.vercel.app';
      const proto = req.headers['x-forwarded-proto'] || 'https';
      const syncUrl = `${proto}://${host}/sync?id=${id}`;

      return res.status(200).json({
        success: true,
        id,
        url: syncUrl
      });
    }

    if (req.method === 'GET') {
      const syncId = req.query.id;
      if (!syncId || typeof syncId !== 'string') {
        return res.status(400).json({ success: false, error: 'Missing sync id parameter (?id=...)' });
      }

      // Query from Supabase table rail_sync by id
      const rows = await querySupabase(
        supabaseUrl,
        supabaseAnon,
        'GET',
        `rail_sync?id=eq.${encodeURIComponent(syncId.trim())}&select=id,encrypted_data,salt,iv,created_at`
      );

      if (!rows || rows.length === 0) {
        return res.status(404).json({ success: false, error: 'Sync data not found or expired.' });
      }

      const row = rows[0];
      return res.status(200).json({
        success: true,
        id: row.id,
        encrypted_data: row.encrypted_data,
        salt: row.salt,
        iv: row.iv,
        created_at: row.created_at
      });
    }

    return res.status(405).json({ success: false, error: 'Method not allowed' });
  } catch (err) {
    return res.status(500).json({
      success: false,
      error: err.message || 'Internal server error'
    });
  }
}
