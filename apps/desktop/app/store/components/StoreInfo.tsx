"use client"
import { Button, Card, CardContent, CardHeader, CardTitle, IApp, IAppVisibility, IMetadata, Separator } from "@tm9657/flow-like-ui";
import { Download, ExternalLink, KeyRound, Play, Settings as SettingsIcon, ShoppingCart, Star, Users } from "lucide-react";
import { visibilityLabel } from "./Visibility";

export function InfoGrid({ app, meta }: Readonly<{ app: IApp; meta: IMetadata }>) {
    const since = new Date((app.updated_at?.secs_since_epoch ?? 0) * 1000);
    const rows = [
        { label: "Category", value: app.primary_category ?? "Other" },
        { label: "Visibility", value: visibilityLabel(app.visibility) },
        { label: "Version", value: app.version || "—" },
        { label: "Updated", value: isNaN(+since) ? "—" : since.toLocaleDateString() },
        { label: "Authors", value: app.authors?.join(", ") || "—" },
        { label: "Website", value: meta.website ? <ExternalLinkRow href={meta.website} /> : "—" },
    ];
    return (
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {rows.map((r) => (
                <div key={r.label} className="flex items-center justify-between rounded-lg border bg-card px-4 py-3">
                    <span className="text-sm text-muted-foreground">{r.label}</span>
                    <span className="text-sm font-medium truncate max-w-[60%] text-right">{r.value as any}</span>
                </div>
            ))}
        </div>
    );
}

export function MediaGallery({ media }: Readonly<{ media: string[] }>) {
    if (!media?.length) return <p className="text-sm text-muted-foreground">No media available.</p>;
    return (
        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
            {media.map((m, i) => (
                <div key={`${m}-${i}`} className="relative overflow-hidden rounded-lg border">
                    <img src={m} alt={`preview-${i}`} className="h-40 w-full object-cover" loading="lazy" decoding="async" />
                </div>
            ))}
        </div>
    );
}

export function DetailsCard({
    app,
    meta,
    isMember,
    price,
    visibility,
    priceLabel,
    onUse,
    onSettings,
    onJoinOrRequest,
    onBuy,
}: Readonly<{
    app: IApp;
    meta: IMetadata;
    isMember: boolean;
    price: number;
    visibility: IAppVisibility;
    priceLabel: string;
    onUse: () => void;
    onSettings: () => void;
    onJoinOrRequest: () => Promise<void> | void;
    onBuy: () => void;
}>) {
    const stats = [
        { icon: Star, label: "Rating", value: app.rating_count > 0 ? `${(app.avg_rating ?? 0).toFixed(1)} (${app.rating_count.toLocaleString()})` : "No ratings" },
        { icon: Users, label: "Members", value: app.interactions_count?.toLocaleString?.() ?? "—" },
        { icon: Download, label: "Downloads", value: app.download_count?.toLocaleString?.() ?? "—" },
    ];

    return (
        <Card>
            <CardHeader>
                {isMember ? (
                    <div className="flex flex-1 items-center gap-3">
                        <Button variant="outline" onClick={onSettings} className="flex-1 md:flex-none md:min-w-[160px]"><SettingsIcon className="h-4 w-4 mr-2" /> Settings</Button>
                        <Button onClick={onUse} className="flex-1 md:flex-none md:min-w-[180px]"> <Play className="h-4 w-4 mr-2" /> Use App</Button>
                    </div>
                ) : (
                    <div className="flex flex-1 items-center gap-3">
                        {price > 0 ? (
                            <>
                                <Button onClick={onJoinOrRequest} className="flex-1 md:flex-none md:min-w-[200px]"><KeyRound className="h-4 w-4 mr-2" /> Request access</Button>
                                <Button variant="secondary" onClick={onBuy} className="flex-1 md:flex-none md:min-w-[160px]"><ShoppingCart className="h-4 w-4 mr-2" /> Buy {priceLabel}</Button>
                            </>
                        ) : visibility === IAppVisibility.Public ? (
                            <Button onClick={onJoinOrRequest} className="flex-1 md:flex-none md:min-w-[200px] w-full"><Users className="h-4 w-4 mr-2" /> Join for free</Button>
                        ) : (
                            <Button onClick={onJoinOrRequest} className="flex-1 md:flex-none md:min-w-[200px] w-full"><KeyRound className="h-4 w-4 mr-2" /> Request access</Button>
                        )}
                    </div>
                )}
                <CardTitle className="mt-4">Details</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
                {stats.map(({ icon: Icon, label, value }) => (
                    <div key={label} className="flex items-center justify-between">
                        <div className="flex items-center gap-2 text-muted-foreground">
                            <Icon className="h-4 w-4" />
                            <span className="text-sm">{label}</span>
                        </div>
                        <span className="text-sm font-medium">{value}</span>
                    </div>
                ))}
                <Separator />
                <LinkRow label="Website" href={meta.website} />
                <LinkRow label="Documentation" href={meta.docs_url} />
                <LinkRow label="Support" href={meta.support_url} />
            </CardContent>
        </Card>
    );
}

export function LinkRow({ label, href }: Readonly<{ label: string; href?: string | null }>) {
    if (!href) return null;
    return (
        <a href={href} target="_blank" rel="noreferrer" className="flex items-center justify-between rounded-md border px-3 py-2 hover:bg-muted/50 transition">
            <span className="text-sm">{label}</span>
            <ExternalLink className="h-4 w-4 text-muted-foreground" />
        </a>
    );
}

export function ExternalLinkRow({ href }: Readonly<{ href: string }>) {
    return (
        <a href={href} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 text-primary hover:underline">
            <span className="truncate max-w-[220px] align-middle">{href}</span>
            <ExternalLink className="h-3.5 w-3.5" />
        </a>
    );
}
