"use client"
import { QueryObserverResult } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { Badge, BitHover, Button, DynamicImage, IBit, IBitTypes, Input, ISettingsProfile, IVault, Label, Progress, Textarea, useInvoke } from "@tm9657/flow-like-ui";
import { useRouter } from "next/navigation";
import { Dispatch, SetStateAction, useEffect, useState } from "react";
import Crossfire from "react-canvas-confetti/dist/presets/crossfire";
import { toast } from "sonner";

const BLANK_BIT: IBit = {
    authors: [],
    created: "",
    dependencies: [],
    dependency_tree_hash: "",
    hash: "",
    hub: "",
    icon: "/templates/blank.svg",
    id: "blank",
    license: "",
    meta: {
        "en": {
            description: "Start from a blank canvas and create your own Vault",
            long_description: "Create your own Vault from scratch",
            name: "Blank",
            tags: [],
            use_case: "Create your own Vault from scratch"
        }
    },
    parameters: {},
    size: 0,
    type: IBitTypes.Template,
    updated: "",
    version: ""
}

interface ICreationDialog {
    templateId: string,
    progress: number,
    tags: string[],
    models: string[],
    name: string,
    description: string,
    author: string
}

export default function CreateVaultPage() {
    const templates = useInvoke<IBit[]>("get_bits_by_category", { bitType: IBitTypes.Template })
    const vaults = useInvoke<IVault[]>("get_vaults", {})
    const currentProfile = useInvoke<ISettingsProfile | null>("get_current_profile", {})
    const [creationDialog, setCreationDialog] = useState<ICreationDialog>({
        templateId: "blank",
        progress: 0,
        models: [],
        tags: [],
        name: "",
        description: "",
        author: ""
    })

    return <main className="flex min-h-screen flex-col w-full p-4 py-8 max-h-[100dvh] overflow-y-auto items-center">
        {creationDialog.progress < 3 && <div className="flex flex-row items-center justify-center w-full">
            <Progress value={100 * (creationDialog.progress + 1) / 3} className="mb-3 max-w-screen-lg h-1" />
        </div>}
        {creationDialog.progress === 0 && <TemplateSelection creationDialog={creationDialog} setCreationDialog={setCreationDialog} templates={templates} />}
        {creationDialog.progress === 1 && <MetadataCreation creationDialog={creationDialog} setCreationDialog={setCreationDialog} />}
        {creationDialog.progress === 2 && <ModelSelection creationDialog={creationDialog} setCreationDialog={setCreationDialog} currentProfile={currentProfile} />}
        {creationDialog.progress === 3 && <FinalScreen creationDialog={creationDialog} setCreationDialog={setCreationDialog} refresh={async () => {await vaults.refetch()}} />}
    </main>
}


function FinalScreen({ creationDialog, setCreationDialog, refresh }: Readonly<{ creationDialog: ICreationDialog, setCreationDialog: Dispatch<SetStateAction<ICreationDialog>>, refresh: () => Promise<void> }>) {
    const router = useRouter()
    return <>
        <Crossfire className="absolute top-0 left-0 right-0 bottom-0 w-full h-full" autorun={{ speed: 1 }} />
        <div className="max-w-screen-lg w-full h-full flex-grow flex flex-col gap-3 max-h-full justify-center items-center overflow-hidden relative">
            <div className="max-w-screen-sm w-full border p-4 bg-background rounded-md flex flex-col gap-4">
                <div>
                    <h2><b className="text-primary">Done</b></h2>
                    <p>Are you happy with your selection?</p>
                </div>
                <div className="w-full flex flex-row justify-end gap-2">
                    <Button variant={"outline"} onClick={() => {
                        setCreationDialog(old => ({ ...old, progress: 2 }))
                    }}>
                        Back
                    </Button>
                    <Button onClick={async () => {
                        await invoke("create_vault", {
                            name: creationDialog.name,
                            description: creationDialog.description,
                            author: creationDialog.author,
                            bits: creationDialog.models,
                            template: creationDialog.templateId,
                            tags: creationDialog.tags
                        })
                        toast("Created Vault 🎉")
                        await refresh()
                        router.push("/vaults")
                    }}>
                        Finish
                    </Button>
                </div>
            </div>
        </div>
    </>
}

function MetadataCreation({ creationDialog, setCreationDialog }: Readonly<{ creationDialog: ICreationDialog, setCreationDialog: Dispatch<SetStateAction<ICreationDialog>> }>) {
    const [localTags, setLocalTags] = useState("")

    useEffect(() => {
        setCreationDialog(old => ({ ...old, tags: localTags.split(",").map(tag => tag.trim().toLowerCase()) }))
    }, [localTags])

    return <div className="max-w-screen-lg w-full h-full flex-grow flex flex-col gap-3 max-h-full overflow-hidden">
        <div>
            <h2><b className="text-primary">2.</b> Let´s name your new Vault</h2>
            <p>The metadata you assign to your vault is not only helpful for other humans to understand the content, but also for AI agents working with it!</p>
        </div>
        <div className="flex-grow p-2 rounded-md max-h-full overflow-y-auto h-full flex flex-col gap-4 bg-background">
            <div className="grid items-center gap-1.5 w-full">
                <Label htmlFor="name">Name</Label>
                <Input value={creationDialog.name} type="text" id="name" placeholder="Name" onChange={(e) => { setCreationDialog(old => ({ ...old, name: e.target.value })) }} />
            </div>
            <div className="grid items-center gap-1.5 w-full">
                <Label htmlFor="description">Description</Label>
                <Textarea value={creationDialog.description} cols={12} id="description" placeholder="Description" onChange={(e) => { setCreationDialog(old => ({ ...old, description: e.target.value })) }} />
            </div>
            <div className="grid items-center gap-1.5 w-full">
                <Label htmlFor="author">Author</Label>
                <Input value={creationDialog.author} type="text" id="author" placeholder="Author" onChange={(e) => { setCreationDialog(old => ({ ...old, author: e.target.value })) }} />
            </div>
            <div className="grid items-center gap-1.5 w-full">
                <Label htmlFor="tags">Tags</Label>
                <Input value={localTags} type="text" id="tags" placeholder="Tags (tag1, tag2)" onChange={(e) => { setLocalTags(e.target.value) }} />
            </div>
            <div className="flex flex-row gap-2">
                {creationDialog.tags.filter(tag => tag !== "").map(tag => <Badge key={tag} variant={"default"}>{tag}</Badge>)}
            </div>
        </div>
        <div className="w-full flex flex-row justify-end gap-2">
            <Button variant={"outline"} onClick={() => {
                setCreationDialog(old => ({ ...old, progress: 0 }))
            }}>
                Back
            </Button>
            <Button disabled={(creationDialog.name === "" || creationDialog.description === "")} onClick={() => {
                setCreationDialog(old => ({ ...old, progress: 2 }))
            }}>
                Continue
            </Button>
        </div>
    </div>
}

function ModelSelection({ creationDialog, setCreationDialog, currentProfile }: Readonly<{ creationDialog: ICreationDialog, setCreationDialog: Dispatch<SetStateAction<ICreationDialog>>, currentProfile: QueryObserverResult<ISettingsProfile | null> }>) {
    return <div className="max-w-screen-lg w-full h-full flex-grow flex flex-col gap-3 max-h-full overflow-hidden">
        <div>
            <h2><b className="text-primary">3.</b> Select the Embedding Models for this Vault</h2>
            <p>You will not be able to change them later on. You would need to recreate the Vault, for other Embedding models.</p>
        </div>
        <div className="flex-grow border p-2 rounded-md bg-background max-h-full overflow-y-auto h-full flex flex-col">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2">
                {currentProfile.data?.hub_profile.bits?.map((bit, i) => <Bit bit={bit[1]} hub={bit[0]} onSelect={() => {
                    setCreationDialog(old => {
                        if (old.models.includes(bit[1])) {
                            return { ...old, models: old.models.filter(m => m !== bit[1]) }
                        }
                        return { ...old, models: [...old.models, bit[1]] }
                    })
                }} selected={creationDialog.models.includes(bit[1])} key={bit[1]} />)}
            </div>
        </div>
        <div className="w-full flex flex-row justify-end gap-2">
            <Button variant={"outline"} onClick={() => {
                setCreationDialog(old => ({ ...old, progress: 1 }))
            }}>
                Back
            </Button>
            <Button disabled={(creationDialog.models.length === 0)} onClick={() => {
                setCreationDialog(old => ({ ...old, progress: 3 }))
            }}>
                Continue
            </Button>
        </div>
    </div>
}

function TemplateSelection({ creationDialog, setCreationDialog, templates }: Readonly<{ creationDialog: ICreationDialog, setCreationDialog: Dispatch<SetStateAction<ICreationDialog>>, templates: QueryObserverResult<IBit[]> }>) {
    return <div className="max-w-screen-lg w-full h-full flex-grow flex flex-col gap-3 max-h-full overflow-hidden">
        <div>
            <h2><b className="text-primary">1.</b> Let´s create your new Vault</h2>
            <p>First, we need to select a starting template for your new Vault. Please select one from below or start from scratch with a blank Template.</p>
        </div>
        <div className="flex-grow border p-2 rounded-md bg-background max-h-full overflow-y-auto h-full flex flex-col">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2">
                <Template bit={BLANK_BIT} onSelect={() => {
                    setCreationDialog(old => ({ ...old, templateId: "blank" }))
                }} selected={creationDialog.templateId === "blank"} />
                {
                    templates.data?.map((bit, i) => {
                        return <Template key={bit.id} selected={creationDialog.templateId === bit.id} bit={bit} onSelect={() => {
                            setCreationDialog(old => ({ ...old, templateId: bit.id }))
                        }} />
                    })
                }
            </div>
        </div>
        <div className="w-full flex flex-row justify-end">
            <Button disabled={creationDialog.templateId === ""} onClick={() => {
                setCreationDialog(old => ({ ...old, progress: 1 }))
            }}>
                Continue
            </Button>
        </div>
    </div>
}

function Template({ bit, selected, onSelect }: Readonly<{ bit: IBit, selected: boolean, onSelect: () => void }>) {
    return (
        <button className={`w-full border p-2 rounded-md flex flex-col gap-2 items-start group hover:bg-primary hover:text-primary-foreground transition-all ${selected ? "text-primary-foreground bg-primary" : ""}`} onClick={onSelect}>
            <div className="flex flex-row items-center gap-3">
                <div className="rounded-full overflow-hidden">
                    {!bit.icon.endsWith("svg") && <img alt={bit.meta?.["en"]?.name} width={32} height={32} src={bit.icon} />}
                    {bit.icon.endsWith("svg") && <DynamicImage url={bit.icon} className={`bg-primary group-hover:bg-primary-foreground w-6 h-6 transition-all ${selected ? "bg-primary-foreground" : ""}`} />}
                </div>
                <p className="line-clamp-1">{bit.meta?.["en"]?.name}</p>
            </div>
            <small className={`text-start text-muted-foreground group-hover:text-primary-foreground transition-all line-clamp-3 ${selected ? "text-primary-foreground" : ""}`}>{bit.meta?.["en"]?.description}</small>
        </button>
    )
}

function Bit({ bit, hub, selected, onSelect }: Readonly<{ bit: string, hub: string, selected: boolean, onSelect: () => void }>) {
    const bitData = useInvoke<IBit>("get_bit_by_id", { bit: [bit, hub], hub }, [bit, hub])

    if (!bitData.data) return null
    if (bitData.data.type !== IBitTypes.Embedding && bitData.data.type !== IBitTypes.ImageEmbedding) return null

    return <BitHover bit={bitData.data}>
        <button className={`w-full h-full flex-grow border p-2 rounded-md flex flex-col gap-2 items-start group hover:bg-primary hover:text-primary-foreground transition-all ${selected && "bg-primary text-primary-foreground"}`} onClick={onSelect}>
            <div className="flex flex-row items-center gap-3">
                <div className="rounded-full overflow-hidden">
                    {!bitData.data.icon.endsWith("svg") && <img alt={bitData.data.meta?.["en"]?.name} width={32} height={32} src={bitData.data.icon} />}
                    {bitData.data.icon.endsWith("svg") && <DynamicImage url={bitData.data.icon} className="bg-primary group-hover:bg-primary-foreground w-6 h-6 transition-all" />}
                </div>
                <p className="line-clamp-1">{bitData.data.meta?.["en"]?.name}</p>
                <small>{bitData.data.type}</small>
            </div>
            <small className={`text-start text-muted-foreground group-hover:text-primary-foreground transition-all line-clamp-3 ${selected && "text-primary-foreground"}`}>{bitData.data.meta?.["en"]?.description}</small>
        </button>
    </BitHover>
}