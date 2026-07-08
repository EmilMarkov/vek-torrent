// Периодический опрос сводного статуса подсистем.

import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api";

export function useStatus() {
  return useQuery({
    queryKey: ["status"],
    queryFn: api.status,
    refetchInterval: 5000,
    refetchOnWindowFocus: false,
  });
}
