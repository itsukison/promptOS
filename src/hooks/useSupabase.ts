import { useState, useEffect, useCallback } from "react";
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

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setSession(session);
      setUser(session?.user ?? null);
    });

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

  const getRemainingTokens = useCallback(async (): Promise<number> => {
    const { data } = await supabase
      .from("user_profiles")
      .select("tokens_remaining")
      .single();
    return data?.tokens_remaining ?? 0;
  }, []);

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
