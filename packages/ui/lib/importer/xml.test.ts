import { describe, expect, test } from "bun:test";
import {
	XmlParseError,
	attr,
	childrenNamed,
	decodeXmlEntities,
	firstChild,
	looksLikeXml,
	parseXml,
	textOf,
	walk,
} from "./xml";

describe("parseXml", () => {
	test("reads a namespaced document with attributes, text and nesting", () => {
		const root = parseXml(`<?xml version="1.0" encoding="UTF-8"?>
<!-- exported -->
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Defs_1">
  <bpmn:process id="P1" isExecutable='true'>
    <bpmn:task id="T1" name="Say &quot;hi&quot; &amp; wave" camunda:assignee="kermit"/>
    <bpmn:documentation>Line one
line two</bpmn:documentation>
  </bpmn:process>
</bpmn:definitions>`);

		expect(root.name).toBe("definitions");
		expect(root.prefix).toBe("bpmn");
		expect(root.attrs.id).toBe("Defs_1");
		const process = firstChild(root, "process");
		if (!process) throw new Error("process missing");
		expect(process.attrs.isExecutable).toBe("true");
		const task = firstChild(process, "task");
		if (!task) throw new Error("task missing");
		expect(task.attrs.name).toBe('Say "hi" & wave');
		expect(attr(task, "assignee", { prefix: "camunda" })).toBe("kermit");
		expect(attr(task, "assignee")).toBeUndefined();
		expect(attr(task, "assignee", { anyPrefix: true })).toBe("kermit");
		expect(textOf(firstChild(process, "documentation"))).toBe(
			"Line one\nline two",
		);
	});

	test("keeps CDATA verbatim and skips comments and PIs inside elements", () => {
		const root = parseXml(
			"<a><script><![CDATA[if (x < 3 && y > 1) { go(); }]]><!-- c --><?pi x?></script></a>",
		);
		expect(firstChild(root, "script")?.text).toBe(
			"if (x < 3 && y > 1) { go(); }",
		);
	});

	test("decodes numeric entities and tolerates unknown ones", () => {
		expect(decodeXmlEntities("&#65;&#x42;&lt;&unknown;")).toBe("AB<&unknown;");
	});

	test("skips a DOCTYPE with an internal subset", () => {
		const root = parseXml(
			"<!DOCTYPE note [<!ELEMENT note (#PCDATA)>]><note>x</note>",
		);
		expect(root.name).toBe("note");
		expect(root.text).toBe("x");
	});

	test("strips a byte order mark", () => {
		expect(parseXml("﻿<r/>").name).toBe("r");
		expect(looksLikeXml("﻿  <r/>")).toBe(true);
		expect(looksLikeXml('{"nodes": []}')).toBe(false);
	});

	test("rejects mismatched and unclosed tags with an offset", () => {
		expect(() => parseXml("<a><b></a>")).toThrow(XmlParseError);
		expect(() => parseXml("<a>")).toThrow(/Unclosed/);
		expect(() => parseXml("<a/><b/>")).toThrow(/after the root/);
		expect(() => parseXml("<a b></a>")).toThrow(/has no value/);
	});

	test("walk visits depth-first and childrenNamed filters direct children", () => {
		const root = parseXml("<r><x id='1'><x id='2'/></x><y/><x id='3'/></r>");
		expect([...walk(root)].map((e) => e.name)).toEqual([
			"r",
			"x",
			"x",
			"y",
			"x",
		]);
		expect(childrenNamed(root, "x").map((e) => e.attrs.id)).toEqual(["1", "3"]);
	});
});
