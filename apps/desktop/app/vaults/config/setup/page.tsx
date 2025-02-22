'use client'

import { IBoard, useInvoke } from '@tm9657/flow-like-ui'
import { useQueryClient, UseQueryResult } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { IVault } from '@tm9657/flow-like-ui'
import { useRouter, useSearchParams } from 'next/navigation'

export default function Id() {
    const searchParams = useSearchParams()
    const queryClient = useQueryClient()
    const router = useRouter()
    const id = searchParams.get('id')
    const vault = useInvoke<IVault>("get_vault", { vaultId: id }, [id ?? ""], typeof id === "string")
    const boards = useInvoke<IBoard[]>("get_vault_boards", { vaultId: id }, [id ?? ""], typeof id === "string")

    async function deleteVault() {
        await invoke("delete_vault", { vaultId: id })
        await queryClient.invalidateQueries({
            queryKey: "get_local_vaults".split("_")
        })
        router.push("/vaults")
    }

    return <main className="justify-start flex flex-col items-start w-full flex-1 max-h-full overflow-y-auto flex-grow gap-4">

    </main>
}