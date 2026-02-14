export interface TextFieldBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface UserProfile {
  id: string;
  tokens_remaining: number;
  tokens_used: number;
  subscription_tier: "free" | "pro";
  created_at: string;
  updated_at: string;
}

export interface UsageLog {
  id: string;
  user_id: string;
  prompt_text: string | null;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  model: string;
  created_at: string;
}
