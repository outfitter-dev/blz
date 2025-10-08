#!/usr/bin/env bash

echo "=== Checking llms.txt fallbacks for sources without llms-full.txt ==="
echo ""

check_url() {
  local id=$1
  local url=$2
  local http_status=$(curl -sL -o /dev/null -w "%{http_code}" "$url" 2>/dev/null)
  printf "%-20s %-6s %s\n" "$id" "$http_status" "$url"
}

# Sources that had 404 on llms-full.txt
check_url "langchain" "https://python.langchain.com/llms.txt"
check_url "nextjs" "https://nextjs.org/llms.txt"
check_url "node" "https://nodejs.org/llms.txt"
check_url "react" "https://react.dev/llms.txt"
check_url "redis" "https://redis.io/llms.txt"
check_url "remix" "https://remix.run/llms.txt"
check_url "shadcn" "https://ui.shadcn.com/llms.txt"
check_url "supabase" "https://supabase.com/llms.txt"
check_url "tailwind" "https://tailwindcss.com/llms.txt"
check_url "typescript" "https://www.typescriptlang.org/llms.txt"

# Sources that had redirects ending in 404
check_url "ai-sdk" "https://sdk.vercel.ai/llms.txt"
check_url "anthropic" "https://docs.anthropic.com/en/llms.txt"
check_url "postgres" "https://www.postgresql.org/llms.txt"
check_url "tanstack" "https://tanstack.com/llms.txt"