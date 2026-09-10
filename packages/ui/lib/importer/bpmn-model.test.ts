import { describe, expect, test } from "bun:test";
import {
	findExtension,
	iso8601DurationToSeconds,
	parseBpmn,
	walkFlowNodes,
	walkSequenceFlows,
} from "./bpmn-model";

const COLLABORATION = `<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL"
  xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
  xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
  xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
  xmlns:camunda="http://camunda.org/schema/1.0/bpmn"
  xmlns:zeebe="http://camunda.org/schema/zeebe/1.0"
  id="Definitions_1" name="Order handling" targetNamespace="http://example.com" exporter="Camunda Modeler" exporterVersion="5.20.0">
  <bpmn:message id="Message_Order" name="orderReceived" />
  <bpmn:error id="Error_Payment" name="PaymentFailed" errorCode="PAY-1" />
  <bpmn:collaboration id="Collab_1">
    <bpmn:participant id="Pool_Shop" name="Shop" processRef="Process_Shop" />
    <bpmn:participant id="Pool_Customer" name="Customer" />
    <bpmn:messageFlow id="MF_1" name="order" sourceRef="Pool_Customer" targetRef="Start_1" messageRef="Message_Order" />
    <bpmn:textAnnotation id="Ann_Collab"><bpmn:text>Whole thing</bpmn:text></bpmn:textAnnotation>
  </bpmn:collaboration>
  <bpmn:process id="Process_Shop" isExecutable="true" camunda:historyTimeToLive="30">
    <bpmn:documentation>Handles an order.</bpmn:documentation>
    <bpmn:laneSet id="LaneSet_1">
      <bpmn:lane id="Lane_Sales" name="Sales">
        <bpmn:flowNodeRef>Start_1</bpmn:flowNodeRef>
        <bpmn:flowNodeRef>Task_Check</bpmn:flowNodeRef>
        <bpmn:childLaneSet id="LaneSet_2">
          <bpmn:lane id="Lane_Inner" name="Inner">
            <bpmn:flowNodeRef>Gateway_1</bpmn:flowNodeRef>
          </bpmn:lane>
        </bpmn:childLaneSet>
      </bpmn:lane>
    </bpmn:laneSet>
    <bpmn:startEvent id="Start_1" name="Order received">
      <bpmn:outgoing>Flow_1</bpmn:outgoing>
      <bpmn:messageEventDefinition id="MED_1" messageRef="Message_Order" />
    </bpmn:startEvent>
    <bpmn:sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="Task_Check" />
    <bpmn:userTask id="Task_Check" name="Check order" camunda:assignee="kermit" camunda:candidateGroups="sales">
      <bpmn:documentation>Look at it.</bpmn:documentation>
      <bpmn:extensionElements>
        <zeebe:formDefinition formKey="check-form" />
        <zeebe:ioMapping>
          <zeebe:input source="=order.id" target="orderId" />
          <zeebe:output source="=approved" target="isApproved" />
        </zeebe:ioMapping>
      </bpmn:extensionElements>
      <bpmn:incoming>Flow_1</bpmn:incoming>
      <bpmn:outgoing>Flow_2</bpmn:outgoing>
      <bpmn:dataInputAssociation id="DIA_1"><bpmn:sourceRef>DataRef_Order</bpmn:sourceRef><bpmn:targetRef>Prop_1</bpmn:targetRef></bpmn:dataInputAssociation>
      <bpmn:dataOutputAssociation id="DOA_1"><bpmn:targetRef>DataRef_Order</bpmn:targetRef></bpmn:dataOutputAssociation>
      <bpmn:multiInstanceLoopCharacteristics isSequential="true">
        <bpmn:extensionElements>
          <zeebe:loopCharacteristics inputCollection="=items" inputElement="item" />
        </bpmn:extensionElements>
        <bpmn:completionCondition>=done</bpmn:completionCondition>
      </bpmn:multiInstanceLoopCharacteristics>
    </bpmn:userTask>
    <bpmn:boundaryEvent id="Boundary_Timer" name="2 days" attachedToRef="Task_Check" cancelActivity="false">
      <bpmn:outgoing>Flow_Timer</bpmn:outgoing>
      <bpmn:timerEventDefinition><bpmn:timeDuration>P2D</bpmn:timeDuration></bpmn:timerEventDefinition>
    </bpmn:boundaryEvent>
    <bpmn:sequenceFlow id="Flow_Timer" sourceRef="Boundary_Timer" targetRef="End_Late" />
    <bpmn:endEvent id="End_Late" name="Late" />
    <bpmn:sequenceFlow id="Flow_2" sourceRef="Task_Check" targetRef="Gateway_1" />
    <bpmn:exclusiveGateway id="Gateway_1" name="Approved?" default="Flow_No">
      <bpmn:incoming>Flow_2</bpmn:incoming>
      <bpmn:outgoing>Flow_Yes</bpmn:outgoing>
      <bpmn:outgoing>Flow_No</bpmn:outgoing>
    </bpmn:exclusiveGateway>
    <bpmn:sequenceFlow id="Flow_Yes" name="yes" sourceRef="Gateway_1" targetRef="Sub_1">
      <bpmn:conditionExpression xsi:type="bpmn:tFormalExpression" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" language="feel"><![CDATA[=approved = true]]></bpmn:conditionExpression>
    </bpmn:sequenceFlow>
    <bpmn:sequenceFlow id="Flow_No" name="no" sourceRef="Gateway_1" targetRef="End_Rejected" />
    <bpmn:subProcess id="Sub_1" name="Fulfil">
      <bpmn:incoming>Flow_Yes</bpmn:incoming>
      <bpmn:outgoing>Flow_3</bpmn:outgoing>
      <bpmn:startEvent id="Sub_Start"><bpmn:outgoing>Sub_Flow</bpmn:outgoing></bpmn:startEvent>
      <bpmn:sequenceFlow id="Sub_Flow" sourceRef="Sub_Start" targetRef="Sub_Script" />
      <bpmn:scriptTask id="Sub_Script" name="Total" scriptFormat="javascript" camunda:resultVariable="total">
        <bpmn:script><![CDATA[var t = a + b;]]></bpmn:script>
      </bpmn:scriptTask>
      <bpmn:callActivity id="Sub_Call" name="Ship">
        <bpmn:extensionElements><zeebe:calledElement processId="Process_Ship" /></bpmn:extensionElements>
      </bpmn:callActivity>
    </bpmn:subProcess>
    <bpmn:sequenceFlow id="Flow_3" sourceRef="Sub_1" targetRef="End_Done" />
    <bpmn:endEvent id="End_Done" name="Done">
      <bpmn:errorEventDefinition errorRef="Error_Payment" />
    </bpmn:endEvent>
    <bpmn:endEvent id="End_Rejected" name="Rejected" />
    <bpmn:dataObject id="Data_Order" />
    <bpmn:dataObjectReference id="DataRef_Order" name="Order" dataObjectRef="Data_Order"><bpmn:dataState name="checked" /></bpmn:dataObjectReference>
    <bpmn:dataStoreReference id="Store_1" name="Orders DB" />
    <bpmn:textAnnotation id="Ann_1"><bpmn:text>Manual step</bpmn:text></bpmn:textAnnotation>
    <bpmn:association id="Assoc_1" sourceRef="Ann_1" targetRef="Task_Check" />
    <bpmn:group id="Group_1" categoryValueRef="CV_1" />
  </bpmn:process>
  <bpmndi:BPMNDiagram id="Diagram_1">
    <bpmndi:BPMNPlane id="Plane_1" bpmnElement="Collab_1">
      <bpmndi:BPMNShape id="Shape_Pool" bpmnElement="Pool_Shop" isHorizontal="true"><dc:Bounds x="100" y="50" width="800" height="300" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Shape_Start" bpmnElement="Start_1"><dc:Bounds x="180" y="150" width="36" height="36" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Shape_Task" bpmnElement="Task_Check"><dc:Bounds x="270" y="128" width="100" height="80" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Shape_Sub" bpmnElement="Sub_1" isExpanded="false"><dc:Bounds x="500" y="128" width="100" height="80" /></bpmndi:BPMNShape>
      <bpmndi:BPMNEdge id="Edge_1" bpmnElement="Flow_1"><di:waypoint x="216" y="168" /><di:waypoint x="270" y="168" /></bpmndi:BPMNEdge>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</bpmn:definitions>`;

describe("parseBpmn", () => {
	const defs = parseBpmn(COLLABORATION);
	const shop = defs.processes[0];
	const byId = new Map([...walkFlowNodes(shop)].map((n) => [n.id, n]));

	test("reads definitions, root definitions and the collaboration", () => {
		expect(defs.name).toBe("Order handling");
		expect(defs.exporter).toBe("Camunda Modeler");
		expect(defs.messages.get("Message_Order")?.name).toBe("orderReceived");
		expect(defs.errors.get("Error_Payment")?.code).toBe("PAY-1");
		expect(defs.collaboration?.participants.map((p) => p.name)).toEqual([
			"Shop",
			"Customer",
		]);
		expect(defs.collaboration?.messageFlows[0]?.messageRef).toBe(
			"Message_Order",
		);
		expect(defs.collaboration?.annotations[0]?.text).toBe("Whole thing");
	});

	test("binds a process to its participant and reads process metadata", () => {
		expect(shop.id).toBe("Process_Shop");
		expect(shop.participantName).toBe("Shop");
		expect(shop.isExecutable).toBe(true);
		expect(shop.documentation).toBe("Handles an order.");
		expect(shop.vendorAttrs["camunda:historyTimeToLive"]).toBe("30");
	});

	test("reads nested lanes", () => {
		expect(shop.lanes).toHaveLength(1);
		expect(shop.lanes[0].flowNodeRefs).toEqual(["Start_1", "Task_Check"]);
		expect(shop.lanes[0].children[0].name).toBe("Inner");
		expect(shop.lanes[0].children[0].flowNodeRefs).toEqual(["Gateway_1"]);
	});

	test("reads events with resolved definitions", () => {
		const start = byId.get("Start_1");
		expect(start?.eventDefinitions).toEqual([
			{ kind: "message", ref: "Message_Order", refName: "orderReceived" },
		]);
		const boundary = byId.get("Boundary_Timer");
		expect(boundary?.attachedToRef).toBe("Task_Check");
		expect(boundary?.interrupting).toBe(false);
		expect(boundary?.eventDefinitions[0]?.timer?.timeDuration).toBe("P2D");
		const end = byId.get("End_Done");
		expect(end?.eventDefinitions[0]).toEqual({
			kind: "error",
			ref: "Error_Payment",
			refName: "PaymentFailed",
			code: "PAY-1",
		});
	});

	test("reads activities with vendor data, io mapping, loops and data associations", () => {
		const task = byId.get("Task_Check");
		if (!task) throw new Error("Task_Check missing");
		expect(task.kind).toBe("userTask");
		expect(task.documentation).toBe("Look at it.");
		expect(task.vendorAttrs["camunda:assignee"]).toBe("kermit");
		expect(findExtension(task, "zeebe", "formDefinition")?.attrs.formKey).toBe(
			"check-form",
		);
		expect(task?.ioMapping).toEqual({
			inputs: [{ source: "=order.id", target: "orderId" }],
			outputs: [{ source: "=approved", target: "isApproved" }],
		});
		expect(task?.loop).toMatchObject({
			kind: "multiInstance",
			isSequential: true,
			inputCollection: "=items",
			inputElement: "item",
			completionCondition: "=done",
		});
		expect(task?.dataInputs).toEqual(["DataRef_Order"]);
		expect(task?.dataOutputs).toEqual(["DataRef_Order"]);
	});

	test("reads gateways, conditions and defaults", () => {
		const gateway = byId.get("Gateway_1");
		expect(gateway?.defaultFlow).toBe("Flow_No");
		expect(gateway?.outgoing).toEqual(["Flow_Yes", "Flow_No"]);
		const yes = shop.sequenceFlows.find((f) => f.id === "Flow_Yes");
		expect(yes?.condition).toBe("=approved = true");
		expect(yes?.conditionLanguage).toBe("feel");
	});

	test("reads sub-process bodies recursively", () => {
		const sub = byId.get("Sub_1");
		expect(sub?.body?.flowNodes.map((n) => n.id)).toEqual([
			"Sub_Start",
			"Sub_Script",
			"Sub_Call",
		]);
		const script = byId.get("Sub_Script");
		expect(script?.script).toEqual({
			format: "javascript",
			body: "var t = a + b;",
			resultVariable: "total",
		});
		expect(byId.get("Sub_Call")?.calledElement).toBe("Process_Ship");
		expect([...walkSequenceFlows(shop)].map((f) => f.id)).toContain("Sub_Flow");
	});

	test("reads data objects, artifacts and diagram geometry", () => {
		expect(shop.dataObjects.map((d) => d.kind)).toEqual([
			"dataObject",
			"dataObjectReference",
			"dataStoreReference",
		]);
		expect(shop.dataObjects[1]).toMatchObject({
			name: "Order",
			ref: "Data_Order",
			state: "checked",
		});
		expect(shop.annotations[0].text).toBe("Manual step");
		expect(shop.associations[0].targetRef).toBe("Task_Check");
		expect(shop.groups[0].categoryValueRef).toBe("CV_1");
		expect(defs.diagram.shapes.get("Task_Check")).toEqual({
			x: 270,
			y: 128,
			width: 100,
			height: 80,
			isExpanded: undefined,
			isHorizontal: undefined,
		});
		expect(defs.diagram.shapes.get("Sub_1")?.isExpanded).toBe(false);
		expect(defs.diagram.shapes.get("Pool_Shop")?.isHorizontal).toBe(true);
		expect(defs.diagram.edges.get("Flow_1")).toHaveLength(2);
	});

	test("rejects non-BPMN XML", () => {
		expect(() => parseBpmn("<svg/>")).toThrow(/Not a BPMN document/);
	});
});

describe("iso8601DurationToSeconds", () => {
	test("parses common durations", () => {
		expect(iso8601DurationToSeconds("PT5M")).toBe(300);
		expect(iso8601DurationToSeconds("P2D")).toBe(172800);
		expect(iso8601DurationToSeconds("P1DT2H30M")).toBe(95400);
		expect(iso8601DurationToSeconds("PT0.5S")).toBe(0.5);
		expect(iso8601DurationToSeconds("R3/PT1H")).toBeUndefined();
		expect(iso8601DurationToSeconds("nonsense")).toBeUndefined();
	});
});
