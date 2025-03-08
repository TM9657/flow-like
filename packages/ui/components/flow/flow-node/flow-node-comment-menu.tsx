import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { Button } from '../../../components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader } from '../../../components/ui/dialog';
import { Textarea } from '../../../components/ui/textarea';
import { type INode } from '../../../lib/schema/flow/node';


export function FlowNodeCommentMenu({ node, boardId, open, onOpenChange }: Readonly<{ node: INode, boardId: string, open: boolean, onOpenChange: (open: boolean) => void }>) {
  const queryClient = useQueryClient()
  const [comment, setComment] = useState("")

  async function saveComment() {
    await invoke("update_node", { boardId: boardId, node: { ...node, comment } })
    onOpenChange(false)
    setComment("")
    refetchBoard()
  }

  async function refetchBoard() {
    queryClient.invalidateQueries({
      queryKey: ["get", "board", boardId]
    })
  }

  return <Dialog open={open} onOpenChange={(open) => {
    onOpenChange(open)
  }}>
    <DialogContent>
      <DialogHeader>
        Comment
      </DialogHeader>
      <DialogDescription>
        <Textarea rows={6} value={comment} onChange={(e) => {
          setComment(e.target.value)
        }} />
      </DialogDescription>
      <DialogFooter>
        <Button onClick={() => { onOpenChange(false) }} variant={"secondary"}>Cancel</Button>
        <Button onClick={async () => await saveComment()}>Save</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
}