"use client"

import { UseQueryResult } from "@tanstack/react-query";
import { humanFileSize, IVault, useInvoke } from "@tm9657/flow-like-ui";
import { Badge, Button, EmptyState, HoverCard, HoverCardContent, HoverCardTrigger, Separator } from "@tm9657/flow-like-ui/components/ui";
import { AlertTriangle, FilesIcon, LinkIcon, Plus, VaultIcon } from "lucide-react";
import Link from "next/link";
import { useRouter } from 'next/navigation';

export default function Page() {
    const vaults: UseQueryResult<IVault[]> = useInvoke("get_vaults", {})
    const router = useRouter()
    return <main className="justify-start flex min-h-dvh max-h-dvh flex-row items-start w-full flex-1 flex-grow p-4">
        <div className="mr-6 max-h-screen overflow-y-auto invisible-scroll flex-2 flex-grow h-full w-full">
            <div className="flex flex-row items-center">
                <h1 className="scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl">Vaults</h1>
                <Link href={"/vaults/new"}>
                    <Button variant="default" className="ml-4"><Plus className="mr-2 h-4 w-4" /> Create Vault</Button>
                </Link>
            </div>
            <Separator className="my-4" />
            <div className="flex flex-row items-center flex-wrap min-h-full flex-grow h-full gap-4">
                {vaults.data?.length === 0 && <EmptyState action={{
                    label: "Create Vault",
                    onClick: () => {
                        router.push("/vaults/new")
                    }
                }} icons={[VaultIcon, FilesIcon, LinkIcon]} className="min-w-full min-h-full flex-grow h-full" title="No Vaults Found" description="Create a custom vault based on your Data for Free and Secure." />}
                {vaults.data?.sort((a, b) => a.updated_at.nanos_since_epoch - b.updated_at.nanos_since_epoch).map((vault, i) => {
                    return <Vault key={vault.id} vault={vault} />
                })}
            </div>
        </div>
    </main>
}

function Vault({ vault }: Readonly<{ vault: IVault }>) {
    const router = useRouter()
    const vault_size = useInvoke<number>("get_vault_size", { vaultId: vault.id }, [vault.id])
    const configured = useInvoke<boolean>("vault_configured", { vaultId: vault.id }, [vault.id])

    return <button onClick={() => {
        router.push(`/vaults/config?id=${vault.id}`)
    }} className="relative p-3 min-h-[190px] max-h-[190px] min-w-[320px] max-w-[320px] flex flex-col justify-start rounded-md border bg-card text-card-foreground hover:border-primary shadow cursor-pointer">
        <h4 className="scroll-m-20 text-start text-md font-semibold tracking-tight line-clamp-1">{vault.name}</h4>
        <div className="flex flex-row items-center flex-wrap gap-2 mt-2">
            {vault.tags.map(tag => <Badge key={tag} variant={"secondary"}>{tag}</Badge>)}
        </div>
        <Separator className="my-3" />
        <div className="">
            <p className="text-xs [&:not(:first-child)]:mt-6 text-start line-clamp-3 text-muted-foreground ">{vault.description}</p>
        </div>
        <div className="absolute bottom-0 right-0 left-0 flex flex-row items-center gap-2 m-2 flex-wrap">
            <Badge variant={"outline"}>{humanFileSize(vault_size.data ?? 0)}</Badge>
            <Badge variant={"outline"}>{vault.author}</Badge>
        </div>

        {!configured.data && !configured.isFetching && <HoverCard>
            <HoverCardTrigger asChild>
                <div className="absolute bottom-0 right-0">
                    <AlertTriangle className="p-1 bg-destructive border rounded-lg w-6 h-6  m-2 text-destructive-foreground" />
                </div>
            </HoverCardTrigger>
            <HoverCardContent className="bg-destructive">
                <p className="text-destructive-foreground text-xs">Setup not complete yet.</p>
            </HoverCardContent>
        </HoverCard>}
    </button>
}