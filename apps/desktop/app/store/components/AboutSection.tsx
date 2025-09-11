"use client";

import {
    type IApp,
    type IMetadata,
    Badge,
    Card,
    CardContent,
    CardHeader,
    CardTitle,
    Separator,
    Tabs,
    TabsContent,
    TabsList,
    TabsTrigger
} from "@tm9657/flow-like-ui";
import { InfoGrid, MediaGallery } from "./StoreInfo";

export function AboutSection({
    app,
    meta,
}: Readonly<{ app: IApp; meta: IMetadata }>) {
    return (
        <Card className="lg:col-span-2 rounded-xl border bg-card flex flex-col h-full min-h-fit">
            <CardHeader>
                <CardTitle>About</CardTitle>
            </CardHeader>
            <CardContent className="space-y-6 flex-1 min-h-0 overflow-hidden">
                <div className="leading-relaxed">
                    <p>{meta.description ?? "No description found."}</p>
                </div>
                {meta.tags?.length ? (
                    <div className="flex flex-wrap gap-2">
                        {meta.tags.map((t) => (
                            <Badge key={t} variant="secondary" className="capitalize">
                                {t}
                            </Badge>
                        ))}
                    </div>
                ) : null}

                <Separator />

                <Tabs defaultValue="overview" className="w-full flex flex-col min-h-fit">
                    <TabsList>
                        <TabsTrigger value="overview">Overview</TabsTrigger>
                        <TabsTrigger value="media">Media</TabsTrigger>
                        <TabsTrigger value="release">Release notes</TabsTrigger>
                    </TabsList>
                    <TabsContent value="overview" className="space-y-4 pt-4 flex-1 min-h-0 overflow-y-auto">
                        <InfoGrid app={app} meta={meta} />
                    </TabsContent>
                    <TabsContent value="media" className="pt-4 flex-1 min-h-0 overflow-y-auto">
                        <MediaGallery media={meta.preview_media || []} />
                    </TabsContent>
                    <TabsContent value="release" className="pt-4 flex-1 min-h-0 overflow-y-auto">
                        {meta.release_notes || app.changelog ? (
                            <pre className="text-sm whitespace-pre-wrap text-muted-foreground bg-muted/30 p-4 rounded-lg">
                                {meta.release_notes || app.changelog}
                            </pre>
                        ) : (
                            <p className="text-sm text-muted-foreground">No release notes available.</p>
                        )}
                    </TabsContent>
                </Tabs>
            </CardContent>
        </Card>
    );
}
