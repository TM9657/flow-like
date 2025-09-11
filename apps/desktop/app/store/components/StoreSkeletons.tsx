"use client"
import { Card, CardContent, CardHeader, CardTitle } from "@tm9657/flow-like-ui";

export function EmptyState({ title, description }: Readonly<{ title: string; description?: string }>) {
  return (
    <Card className="text-center">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent className="text-muted-foreground">{description}</CardContent>
    </Card>
  );
}

export function HeroSkeleton() {
  return (
    <div className="relative overflow-hidden rounded-2xl border bg-card">
      <div className="h-40 md:h-56 w-full bg-muted animate-pulse" />
      <div className="p-6 md:p-8 -mt-12 flex items-end gap-4">
        <div className="h-24 w-24 rounded-full border bg-muted animate-pulse" />
        <div className="space-y-3">
          <div className="h-7 w-48 bg-muted animate-pulse rounded" />
          <div className="h-4 w-72 bg-muted animate-pulse rounded" />
        </div>
      </div>
    </div>
  );
}
