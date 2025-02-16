"use client"
import { FlowWrapper } from '@tm9657/flow-like-ui/components/flow/flow-wrapper';
import '@xyflow/react/dist/style.css';
import { useSearchParams } from 'next/navigation';

export default function FlowEditPage() {
  const searchParams = useSearchParams()
  const boardId = searchParams.get('id') ?? ""

  if(boardId === "") return <p>Board not found...</p>
  return <FlowWrapper boardId={boardId} />
}

