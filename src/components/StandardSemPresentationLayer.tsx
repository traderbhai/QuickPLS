import { Panel, ViewportPortal } from "@xyflow/react";
import { useEffect, useId, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import type {
  StandardSemPresentationLayoutObject,
  StandardSemPresentationLayoutV1,
} from "../types";

export interface StandardSemPresentationLayerProps {
  presentation: StandardSemPresentationLayoutV1;
  editable: boolean;
  onChange: (presentation: StandardSemPresentationLayoutV1) => void;
}

type PresentationKind = StandardSemPresentationLayoutObject["kind"];

export function commitStandardSemPresentationRequiredText(
  draft: string,
  current: string,
  commit: (value: string) => void,
): boolean {
  const normalized = draft.trim();
  if (!normalized) return false;
  if (normalized !== current) commit(normalized);
  return true;
}

export function createStandardSemPresentationObject(
  kind: PresentationKind,
  id: string,
  offset = 0,
): StandardSemPresentationLayoutObject {
  const x = 80 + offset * 18;
  const y = 80 + offset * 18;
  if (kind === "caption") return { kind, id, text: "Caption", x, y };
  if (kind === "note") return { kind, id, subject: "Note", text: "Add note text", x, y };
  if (kind === "shape") return { kind, id, shape: "rounded_rectangle", x, y, width: 180, height: 96, label: "Shape", style: {} };
  if (kind === "image") return { kind, id, assetRef: "asset:replace-me", altText: "Describe this image", x, y, width: 180, height: 120, style: {} };
  return { kind, id, x1: x, y1: y, x2: x + 180, y2: y, label: "Line", startMarker: null, endMarker: "arrow", style: {} };
}

export function moveStandardSemPresentationObject(
  object: StandardSemPresentationLayoutObject,
  dx: number,
  dy: number,
): StandardSemPresentationLayoutObject {
  return object.kind === "line"
    ? { ...object, x1: object.x1 + dx, y1: object.y1 + dy, x2: object.x2 + dx, y2: object.y2 + dy }
    : { ...object, x: object.x + dx, y: object.y + dy };
}

const replaceObject = (
  presentation: StandardSemPresentationLayoutV1,
  replacement: StandardSemPresentationLayoutObject,
): StandardSemPresentationLayoutV1 => ({
  schemaVersion: 1,
  objects: presentation.objects.map((object) => object.id === replacement.id ? replacement : object),
});

const objectLabel = (object: StandardSemPresentationLayoutObject) => {
  if (object.kind === "caption") return `Caption: ${object.text}`;
  if (object.kind === "note") return `Note: ${object.subject}`;
  if (object.kind === "shape") return `${object.shape.replaceAll("_", " ")} shape: ${object.label ?? "unlabelled"}`;
  if (object.kind === "image") return `Image: ${object.altText}`;
  return `Line: ${object.label ?? "unlabelled"}${object.endMarker ? `, end marker ${object.endMarker}` : ""}`;
};

export function StandardSemPresentationLayer({ presentation, editable, onChange }: StandardSemPresentationLayerProps) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const counter = useRef(0);
  const selected = presentation.objects.find((object) => object.id === selectedId) ?? null;
  const commit = (replacement: StandardSemPresentationLayoutObject) => onChange(replaceObject(presentation, replacement));
  const add = (kind: PresentationKind) => {
    const next = ++counter.current;
    const object = createStandardSemPresentationObject(kind, `presentation:${kind}:${Date.now()}:${next}`, presentation.objects.length);
    onChange({ schemaVersion: 1, objects: [...presentation.objects, object] });
    setSelectedId(object.id);
  };
  const remove = (id: string) => {
    onChange({ schemaVersion: 1, objects: presentation.objects.filter((object) => object.id !== id) });
    setSelectedId((current) => current === id ? null : current);
  };
  const keyObject = (event: KeyboardEvent<HTMLButtonElement>, object: StandardSemPresentationLayoutObject) => {
    if (!editable) return;
    const delta = event.shiftKey ? 1 : 10;
    const movement = {
      ArrowLeft: [-delta, 0],
      ArrowRight: [delta, 0],
      ArrowUp: [0, -delta],
      ArrowDown: [0, delta],
    }[event.key];
    if (movement) {
      event.preventDefault();
      commit(moveStandardSemPresentationObject(object, movement[0], movement[1]));
    } else if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      remove(object.id);
    }
  };

  return <>
    <Panel position="top-left" className="standard-sem-presentation-toolbar">
      <div role="toolbar" aria-label="Presentation-only diagram objects" style={{ display: "flex", gap: 4, flexWrap: "wrap", maxWidth: 430 }}>
        {(["caption", "note", "shape", "image", "line"] as const).map((kind) => <button
          key={kind}
          type="button"
          disabled={!editable}
          aria-label={`Add presentation-only ${kind}`}
          onClick={() => add(kind)}
        >Add {kind}</button>)}
      </div>
      <small>Presentation only · excluded from scientific hash</small>
      {selected && editable ? <PresentationEditor object={selected} commit={commit} remove={() => remove(selected.id)} /> : null}
    </Panel>
    <ViewportPortal>
      <div aria-label="Presentation-only diagram layer">
        {presentation.objects.map((object) => {
          const selectedObject = object.id === selectedId;
          const common: CSSProperties = {
            position: "absolute",
            zIndex: selectedObject ? 4 : 1,
            outline: selectedObject ? "3px solid currentColor" : "1px solid currentColor",
            background: object.kind === "note" ? "#fff8c5" : "rgba(255,255,255,.92)",
            color: "#17212b",
            cursor: editable ? "pointer" : "default",
          };
          if (object.kind === "line") {
            const length = Math.hypot(object.x2 - object.x1, object.y2 - object.y1);
            const angle = Math.atan2(object.y2 - object.y1, object.x2 - object.x1) * 180 / Math.PI;
            return <button
              key={object.id}
              type="button"
              className="nodrag nopan"
              aria-label={objectLabel(object)}
              aria-pressed={selectedObject}
              onClick={() => setSelectedId(object.id)}
              onKeyDown={(event) => keyObject(event, object)}
              style={{ ...common, left: object.x1, top: object.y1, width: length, height: 4, transformOrigin: "0 50%", transform: `rotate(${angle}deg)`, border: 0, borderTop: "3px solid currentColor", background: "transparent" }}
            ><span style={{ position: "absolute", transform: `rotate(${-angle}deg)`, left: length / 2, top: 6, whiteSpace: "nowrap" }}>Line · {object.label ?? "unlabelled"}{object.endMarker ? " →" : ""}</span></button>;
          }
          const style: CSSProperties = { ...common, left: object.x, top: object.y };
          if (object.kind === "shape") {
            Object.assign(style, { width: object.width, height: object.height, borderRadius: object.shape === "ellipse" ? "50%" : object.shape === "rounded_rectangle" ? 18 : 0, transform: object.shape === "diamond" ? "rotate(45deg)" : undefined });
          } else if (object.kind === "image") Object.assign(style, { width: object.width, height: object.height, border: "2px dashed currentColor" });
          else Object.assign(style, { minWidth: object.kind === "caption" ? 120 : 180, maxWidth: 260, padding: object.kind === "caption" ? "4px 8px" : 10, textAlign: "left" });
          return <button
            key={object.id}
            type="button"
            className="nodrag nopan"
            aria-label={objectLabel(object)}
            aria-pressed={selectedObject}
            onClick={() => setSelectedId(object.id)}
            onKeyDown={(event) => keyObject(event, object)}
            style={style}
          >
            <strong>{object.kind === "caption" ? object.text : object.kind === "note" ? object.subject : object.kind === "shape" ? `Shape · ${object.label ?? "unlabelled"}` : `Image · ${object.altText}`}</strong>
            {object.kind === "note" ? <span style={{ display: "block" }}>{object.text}</span> : null}
            {object.kind === "image" ? <small style={{ display: "block" }}>{object.assetRef}</small> : null}
          </button>;
        })}
      </div>
    </ViewportPortal>
  </>;
}

function PresentationEditor({ object, commit, remove }: {
  object: StandardSemPresentationLayoutObject;
  commit: (object: StandardSemPresentationLayoutObject) => void;
  remove: () => void;
}) {
  return <fieldset style={{ marginTop: 6, maxWidth: 430 }}>
    <legend>Edit selected {object.kind}</legend>
    {object.kind === "caption" ? <label>Caption <input value={object.text} onChange={(event) => commit({ ...object, text: event.target.value })} /></label> : null}
    {object.kind === "note" ? <><label>Subject <input value={object.subject} onChange={(event) => commit({ ...object, subject: event.target.value })} /></label><label>Note <textarea value={object.text} onChange={(event) => commit({ ...object, text: event.target.value })} /></label></> : null}
    {object.kind === "shape" ? <><label>Shape <select value={object.shape} onChange={(event) => commit({ ...object, shape: event.target.value as typeof object.shape })}><option value="rectangle">Rectangle</option><option value="rounded_rectangle">Rounded rectangle</option><option value="ellipse">Ellipse</option><option value="diamond">Diamond</option></select></label><label>Label <input value={object.label ?? ""} onChange={(event) => commit({ ...object, label: event.target.value || null })} /></label></> : null}
    {object.kind === "image" ? <>
      <RequiredTextEditor
        label="Asset reference"
        value={object.assetRef}
        onCommit={(assetRef) => commit({ ...object, assetRef })}
      />
      <RequiredTextEditor
        label="Alternative text"
        value={object.altText}
        onCommit={(altText) => commit({ ...object, altText })}
      />
    </> : null}
    {object.kind === "line" ? <><label>Label <input value={object.label ?? ""} onChange={(event) => commit({ ...object, label: event.target.value || null })} /></label><label>End marker <select value={object.endMarker ?? "none"} onChange={(event) => commit({ ...object, endMarker: event.target.value === "none" ? null : event.target.value })}><option value="none">None</option><option value="arrow">Arrow</option><option value="circle">Circle</option></select></label></> : null}
    <button type="button" onClick={remove}>Delete selected {object.kind}</button>
  </fieldset>;
}

function RequiredTextEditor({ label, value, onCommit }: {
  label: string;
  value: string;
  onCommit: (value: string) => void;
}) {
  const [draft, setDraft] = useState(value);
  const messageId = useId();
  const valid = draft.trim().length > 0;
  useEffect(() => setDraft(value), [value]);
  const commitDraft = () => commitStandardSemPresentationRequiredText(draft, value, onCommit);
  return <label>
    {label}{" "}
    <input
      value={draft}
      required
      aria-invalid={!valid}
      aria-describedby={!valid ? messageId : undefined}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commitDraft}
      onKeyDown={(event) => {
        if (event.key === "Enter") event.currentTarget.blur();
        if (event.key === "Escape") {
          event.preventDefault();
          setDraft(value);
        }
      }}
    />
    {!valid ? <span id={messageId} role="alert">{label} is required; the saved value is unchanged.</span> : null}
  </label>;
}
