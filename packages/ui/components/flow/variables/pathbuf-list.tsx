"use client"
import { type ColumnDef } from "@tanstack/react-table"
import { ArrowUpDown } from "lucide-react"
import { Button } from "../../../components/ui/button"
import { DataTable } from "../../../components/ui/data-table"
import { HoverCard, HoverCardContent, HoverCardTrigger } from "../../../components/ui/hover-card"
import { type IFileMetadata } from "../../../lib/schema/files/file-metadata"
import { humanFileSize } from "../../../lib/utils"

export const columns: ColumnDef<IFileMetadata>[] = [
    {
      accessorKey: "file_name",
      header: ({ column }) => {
        return (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
          >
            Name
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        )
      },
      cell: ({row}) => {
        return <p className="w-full">{row.getValue("file_name")}</p>
      }
    },
    {
      accessorKey: "file_path",
      header: ({ column }) => {
        return (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
          >
            Path
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        )
      },
      cell: ({row}) => {
        const path: string = row.getValue("file_path")

        return <HoverCard>
            <HoverCardTrigger>
                <p className="max-w-20 overflow-ellipsis overflow-hidden line-clamp-1">{path}</p>
            </HoverCardTrigger>
            <HoverCardContent>
                <p>{path}</p>
            </HoverCardContent>
        </HoverCard> 
      }
    },
    {
      accessorKey: "file_extension",
      header: ({ column }) => {
        return (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
          >
            Extension
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        )
      },
    },
    {
      accessorKey: "file_size",
      header: ({ column }) => {
        return (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
          >
            Size
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        )
      },
      cell: ({row}) => {
        const bytes: number = row.getValue("file_size")

        return <p>{humanFileSize(bytes)}</p>
      }
    },
    {
      accessorKey: "mime_type",
      header: ({ column }) => {
        return (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
          >
            Mime Type
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        )
      },
    },
  ]



export function FileList({ files, children, onFileDelete }: Readonly<{ files: IFileMetadata[], children?: React.ReactNode, onFileDelete?: (file: IFileMetadata) => void }>) {
    const columnsDeleteFile: ColumnDef<IFileMetadata>[] = [...columns, {
      accessorKey: "delete",
      header: () => {
        return <p>Delete</p>
      },
      cell: ({row}) => {
        return <Button onClick={() => {
            if (onFileDelete) {
                onFileDelete(row.original)
            }
        }}><p>Delete</p></Button>
      }
    }]

    return (
        <div className="w-full mb-5 mt-2">
            <DataTable columns={typeof onFileDelete === "function" ? columnsDeleteFile : columns} data={files} >
                {children}
            </DataTable>
        </div>
    )
}