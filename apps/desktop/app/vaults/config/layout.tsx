'use client'

import { invoke } from '@tauri-apps/api/core'
import {
    Badge, Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
    Button,
    HoverCard, HoverCardContent, HoverCardTrigger, humanFileSize, IBoard, INode, IVault,
    Separator,
    useInvoke,
    toastError,
    useRunExecutionStore,
    IRun
} from '@tm9657/flow-like-ui'
import { AlertTriangle, PlayCircleIcon, Vault } from 'lucide-react'
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
    const vault = useInvoke<IVault | undefined>("get_vault", { vaultId: id }, [id ?? ""], typeof id === "string")
    const vaultSize = useInvoke<number>("get_vault_size", { vaultId: id }, [id ?? ""], typeof id === "string")
    const boards = useInvoke<IBoard[]>("get_vault_boards", { vaultId: id }, [id ?? ""], typeof id === "string")
    const { addRun, removeRun } = useRunExecutionStore()

    async function executeBoard(boardId: string, node: INode) {
        await invoke("get_vault_board", { vaultId: id, boardId: boardId, pushToRegistry: true })
        const runId: string | undefined = await invoke("create_run", { boardId: boardId, startIds: [node.id], logLevel: "Debug" })
        if (!runId) {
            toastError("Failed to execute board", <PlayCircleIcon className="w-4 h-4" />)
            return
        }
        await addRun(runId, boardId, [node.id])
        await invoke("execute_run", { id: runId })
        removeRun(runId)
        await invoke("get_run", { id: runId })
        await invoke("finalize_run", { id: runId })
    }

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
                <Separator className='my-2 w-[95%]' />
                <div id="actions" className='w-full pr-5 flex flex-col items-stretch gap-2'>
                    {boards.data?.map(board => Object.values(board.nodes).filter(node => node.start).map(node => [board, node])).flat().map(([board, node]) => <HoverCard key={node.id} openDelay={10} closeDelay={10}>
                        <HoverCardTrigger asChild>
                            <Button variant={"outline"} key={node.id} onClick={async () => {
                                await executeBoard(board.id, node as INode)
                            }}>
                                {node.friendly_name}
                            </Button>
                        </HoverCardTrigger>
                        <HoverCardContent side='right'>
                            <p>{board.name}</p>
                            <small className='text-muted-foreground'>{board.description}</small>
                            <small className='text-muted-foreground'>{node.comment}</small>
                        </HoverCardContent>
                    </HoverCard>)}
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