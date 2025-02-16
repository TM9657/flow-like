'use client'

import {
    Badge, Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
    HoverCard, HoverCardContent, HoverCardTrigger, humanFileSize, IVault,
    useInvoke
} from '@tm9657/flow-like-ui'
import { AlertTriangle, Vault } from 'lucide-react'
import Link from 'next/link'
import { usePathname, useSearchParams } from 'next/navigation'
import { Suspense } from 'react'

export default function Id({
    children,
}: Readonly<{
    children: React.ReactNode;
}>) {
    const searchParams = useSearchParams()
    const id = searchParams.get('id')
    const currentRoute = usePathname();
    const isReady = useInvoke<boolean>("vault_configured", { vaultId: id }, [id ?? ""], typeof id === "string")
    const vault = useInvoke<IVault | undefined>("get_vault", { vaultId: id }, [id ?? ""], typeof id === "string")
    const vaultSize = useInvoke<number>("get_vault_size", { vaultId: id }, [id ?? ""], typeof id === "string")

    return <main className="lex min-h-screen max-h-screen overflow-hidden flex-col w-full p-4 px-6 bg-background flex">
        <Breadcrumb>
            <BreadcrumbList>
                <BreadcrumbItem>
                    <BreadcrumbLink href="/">Home</BreadcrumbLink>
                </BreadcrumbItem>
                <BreadcrumbSeparator />
                <BreadcrumbItem>
                    <BreadcrumbLink href="/vaults">Vaults</BreadcrumbLink>
                </BreadcrumbItem>
                <BreadcrumbSeparator />
                <BreadcrumbItem>
                    <BreadcrumbPage>{vault.data?.name}</BreadcrumbPage>
                </BreadcrumbItem>
            </BreadcrumbList>
        </Breadcrumb>
        <div className="grid w-full gap-1 mt-2">
            <div className='flex flex-row items-center gap-2'>
                <Vault />
                <h1 className="text-3xl font-semibold flex flex-row items-center">{vault.data?.name}</h1>
                <Badge variant={"outline"}>{humanFileSize(vaultSize.data ?? 0)}</Badge>
                {vault.data?.tags.map(tag => <Badge key={tag} variant={"secondary"}>{tag}</Badge>)}
                {!isReady.data && !isReady.isFetching && <HoverCard>
                    <HoverCardTrigger asChild>
                        <AlertTriangle className="p-1 bg-destructive border rounded-lg w-6 h-6 text-destructive-foreground" />
                    </HoverCardTrigger>
                    <HoverCardContent className="bg-destructive">
                        <p className="text-destructive-foreground text-xs">Setup not complete yet.</p>
                    </HoverCardContent>
                </HoverCard>}
            </div>

            <p className="leading-7 line-clamp-1">
                {vault.data?.description}
            </p>
        </div>
        <div className="grid w-full items-start gap-6 md:grid-cols-[180px_1fr] lg:grid-cols-[250px_1fr] mt-8 h-full flex-grow overflow-auto">
            <nav
                className="flex flex-col gap-4 text-sm text-muted-foreground border-r h-full"
            >
                <Link href={`/vaults/config?id=${vault.data?.id}`} className={currentRoute.endsWith("/config") ? "font-semibold text-primary" : ""}>
                    General
                </Link>
                <Link href={`/vaults/config/setup?id=${vault.data?.id}`} className={currentRoute.endsWith("/setup") ? "font-semibold text-primary" : ""}>
                    Setup
                </Link>
                <Link href={`/vaults/config/logic?id=${vault.data?.id}`} className={currentRoute.endsWith("/logic") ? "font-semibold text-primary" : ""}>
                    Logic
                </Link>
                <Link href={`/vaults/config/storage?id=${vault.data?.id}`} className={currentRoute.endsWith("/storage") ? "font-semibold text-primary" : ""}>
                    Storage
                </Link>
                <Link href={`/vaults/config/explore?id=${vault.data?.id}`} className={currentRoute.endsWith("/explore") ? "font-semibold text-primary" : ""}>
                    Explore Data
                </Link>
                <Link href={`/vaults/config/share?id=${vault.data?.id}`} className={currentRoute.endsWith("/share") ? "font-semibold text-primary" : ""}>
                    Share
                </Link>
                <Link href={`/vaults/config/endpoints?id=${vault.data?.id}`} className={currentRoute.endsWith("/evaluation") ? "font-semibold text-primary" : ""}>
                    Endpoints
                </Link>
                <Link href={`/vaults/config/export?id=${vault.data?.id}`} className={currentRoute.endsWith("/export") ? "font-semibold text-primary" : ""}>
                    Export / Import
                </Link>
                <div id="actions" className='w-full pr-5 flex flex-col items-stretch gap-2 mt-2'>
                </div>
            </nav>
            <div className="pb-4 pl-2 flex-grow max-h-full h-full overflow-y-auto">
                <Suspense>
                    {children}
                </Suspense>
            </div>
        </div>
    </main>
}