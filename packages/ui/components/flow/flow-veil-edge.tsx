import {
	BaseEdge,
	type EdgeProps,
	getBezierPath,
	getSmoothStepPath,
	getStraightPath,
} from "@xyflow/react";

export function FlowVeilEdge(props: EdgeProps) {
	const {
		id,
		sourceX,
		sourceY,
		targetX,
		targetY,
		sourcePosition,
		targetPosition,
		style = {},
		markerEnd,
		selected,
		data,
	} = props;

	// Determine path type based on connection mode or default to bezier
	const pathType = data?.pathType || "default";
	let edgePath: string;

	if (pathType === "straight") {
		[edgePath] = getStraightPath({
			sourceX,
			sourceY,
			targetX,
			targetY,
		});
	} else if (pathType === "step" || pathType === "smoothstep") {
		[edgePath] = getSmoothStepPath({
			sourceX,
			sourceY,
			targetX,
			targetY,
			sourcePosition,
			targetPosition,
		});
	} else {
		[edgePath] = getBezierPath({
			sourceX,
			sourceY,
			targetX,
			targetY,
			sourcePosition,
			targetPosition,
		});
	}

	const baseColor = style.stroke || "var(--pin-fn-ref)";
	const isSelected = selected ?? false;

	return (
		<>
			<BaseEdge
				id={`${id}-base`}
				path={edgePath}
				style={{
					stroke: baseColor,
					strokeWidth: isSelected ? 4 : 2,
					opacity: isSelected ? 0.35 : 0.2,
					strokeLinecap: "round",
					strokeLinejoin: "round",
					transition: "all 0.2s ease-in-out",
				}}
			/>

			<BaseEdge
				id={`${id}-animated`}
				path={edgePath}
				markerEnd={markerEnd}
				style={{
					stroke: baseColor,
					strokeWidth: isSelected ? 2.5 : 1.5,
					opacity: isSelected ? 0.7 : 0.5,
					strokeDasharray: "8,8",
					strokeLinecap: "round",
					animation: "dash-flow 20s linear infinite",
					transition: "all 0.2s ease-in-out",
				}}
			/>
		</>
	);
}
