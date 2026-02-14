# Backend — Prompt OS

> Part of the [Prompt OS PRD](./PRD.md). Read this when working on database, authentication, or usage tracking.

---

## Database Schema (Single Source of Truth)

```sql
-- User profiles extension
CREATE TABLE user_profiles (
    id UUID PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
    tokens_remaining INTEGER DEFAULT 100000,
    tokens_used INTEGER DEFAULT 0,
    subscription_tier TEXT DEFAULT 'free' CHECK (subscription_tier IN ('free', 'pro')),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Usage logs
CREATE TABLE usage_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE,
    prompt_text TEXT,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER GENERATED ALWAYS AS (prompt_tokens + completion_tokens) STORED,
    model TEXT DEFAULT 'gemini-3-flash-preview',
    created_at TIMESTAMP DEFAULT NOW()
);

-- Row Level Security
ALTER TABLE user_profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE usage_logs ENABLE ROW LEVEL SECURITY;

-- SELECT policies
CREATE POLICY "Users can view own profile"
    ON user_profiles FOR SELECT
    USING (auth.uid() = id);

CREATE POLICY "Users can view own logs"
    ON usage_logs FOR SELECT
    USING (auth.uid() = user_id);

-- INSERT policies
CREATE POLICY "Users can insert own logs"
    ON usage_logs FOR INSERT
    WITH CHECK (auth.uid() = user_id);

-- UPDATE policies
CREATE POLICY "Users can update own profile"
    ON user_profiles FOR UPDATE
    USING (auth.uid() = id)
    WITH CHECK (auth.uid() = id);

-- Trigger: create profile on signup
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO public.user_profiles (id)
    VALUES (NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE TRIGGER on_auth_user_created
    AFTER INSERT ON auth.users
    FOR EACH ROW EXECUTE FUNCTION public.handle_new_user();
```

---

## Token Enforcement (Server-Side)

> **Why server-side?** Client-side checks can be bypassed. This atomic database function ensures tokens are always properly decremented.

```sql
CREATE OR REPLACE FUNCTION public.consume_tokens(
    p_prompt_tokens INTEGER,
    p_completion_tokens INTEGER,
    p_model TEXT DEFAULT 'gemini-3-flash-preview'
)
RETURNS BOOLEAN AS $$
DECLARE
    v_total_tokens INTEGER;
    v_current_remaining INTEGER;
BEGIN
    v_total_tokens := p_prompt_tokens + p_completion_tokens;
    
    SELECT tokens_remaining INTO v_current_remaining
    FROM user_profiles
    WHERE id = auth.uid()
    FOR UPDATE; -- Lock row for atomic update
    
    IF v_current_remaining IS NULL OR v_current_remaining < v_total_tokens THEN
        RETURN FALSE;
    END IF;
    
    UPDATE user_profiles
    SET tokens_remaining = tokens_remaining - v_total_tokens,
        tokens_used = tokens_used + v_total_tokens,
        updated_at = NOW()
    WHERE id = auth.uid();
    
    INSERT INTO usage_logs (user_id, prompt_tokens, completion_tokens, model)
    VALUES (auth.uid(), p_prompt_tokens, p_completion_tokens, p_model);
    
    RETURN TRUE;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
```

---

## TypeScript Supabase Client

```typescript
// src/lib/supabase.ts
import { createClient } from "@supabase/supabase-js";

const SUPABASE_URL = import.meta.env.VITE_SUPABASE_URL;
const SUPABASE_ANON_KEY = import.meta.env.VITE_SUPABASE_ANON_KEY;

export const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY);
```

### Auth Hook

```typescript
// src/hooks/useSupabase.ts
import { useState, useEffect } from "react";
import { supabase } from "../lib/supabase";
import type { User, Session } from "@supabase/supabase-js";

export function useSupabase() {
  const [user, setUser] = useState<User | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      setSession(session);
      setUser(session?.user ?? null);
      setLoading(false);
    });

    const { data: { subscription } } = supabase.auth.onAuthStateChange(
      (_event, session) => {
        setSession(session);
        setUser(session?.user ?? null);
      }
    );

    return () => subscription.unsubscribe();
  }, []);

  const signIn = async (email: string, password: string) => {
    return supabase.auth.signInWithPassword({ email, password });
  };

  const signUp = async (email: string, password: string) => {
    return supabase.auth.signUp({ email, password });
  };

  const signOut = async () => {
    return supabase.auth.signOut();
  };

  const getRemainingTokens = async (): Promise<number> => {
    const { data } = await supabase
      .from("user_profiles")
      .select("tokens_remaining")
      .single();
    return data?.tokens_remaining ?? 0;
  };

  const consumeTokens = async (
    promptTokens: number,
    completionTokens: number
  ): Promise<boolean> => {
    const { data } = await supabase.rpc("consume_tokens", {
      p_prompt_tokens: promptTokens,
      p_completion_tokens: completionTokens,
      p_model: "gemini-3-flash-preview",
    });
    return data ?? false;
  };

  return {
    user,
    session,
    loading,
    signIn,
    signUp,
    signOut,
    getRemainingTokens,
    consumeTokens,
  };
}
```

### Environment Variables

```bash
# .env (not committed to git)
VITE_SUPABASE_URL=https://your-project.supabase.co
VITE_SUPABASE_ANON_KEY=your-anon-key
```

> **Note**: The Gemini API key is NOT stored in env vars. It's stored in macOS Keychain via the Rust `keychain.rs` module (see [features.md](./features.md)).
