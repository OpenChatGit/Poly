// AITest Tools Module - Canvas API
// Tools for AI to interact with the canvas

// =============================================================================
// AGENT LOOP STATE
// =============================================================================

let agentLoopActive = false;
let agentLoopTask = '';

function isAgentLoopActive() {
  return agentLoopActive;
}

function startAgentLoop(task) {
  // If already active, ignore duplicate start_task calls
  if (agentLoopActive) {
    console.log('[Agent] Already in loop, ignoring start_task');
    return { success: true, message: 'Already working on task', alreadyActive: true };
  }
  agentLoopActive = true;
  agentLoopTask = task || 'Working...';
  console.log('[Agent] Loop started:', agentLoopTask);
  return { success: true, message: `Started: ${agentLoopTask}` };
}

function endAgentLoop(summary) {
  const wasActive = agentLoopActive;
  agentLoopActive = false;
  const task = agentLoopTask;
  agentLoopTask = '';
  console.log('[Agent] Loop ended:', summary);
  return { success: true, task, summary: summary || 'Task completed', wasActive };
}

// =============================================================================
// TOOL DEFINITIONS (for AI)
// =============================================================================

const TOOL_DEFINITIONS = [
  {
    name: 'start_task',
    description: 'Start a multi-step task. Call this BEFORE doing multiple tool calls. This keeps you active until you call end_task.',
    parameters: {
      type: 'object',
      properties: {
        task: { type: 'string', description: 'Brief description of what you will do, e.g. "Fixing 3 errors"' }
      },
      required: ['task']
    }
  },
  {
    name: 'end_task',
    description: 'End a multi-step task. Call this when you are DONE with all fixes/changes. Include a summary for the user.',
    parameters: {
      type: 'object',
      properties: {
        summary: { type: 'string', description: 'Summary of what was done, e.g. "Fixed 3 errors on lines 4, 9, and 15"' }
      },
      required: ['summary']
    }
  },
  {
    name: 'check_diagnostics',
    description: 'Check the current code for errors and warnings. Returns list of issues with line numbers.',
    parameters: {
      type: 'object',
      properties: {},
      required: []
    }
  },
  {
    name: 'edit_canvas',
    description: 'Edit specific lines in the code. Use for targeted fixes.',
    parameters: {
      type: 'object',
      properties: {
        start_line: { type: 'number', description: 'Starting line number (1-based)' },
        end_line: { type: 'number', description: 'Ending line number (1-based)' },
        new_code: { type: 'string', description: 'New code to replace the lines' }
      },
      required: ['start_line', 'end_line', 'new_code']
    }
  },
  {
    name: 'read_canvas',
    description: 'Read the current code from the editor.',
    parameters: {
      type: 'object',
      properties: {},
      required: []
    }
  },
  {
    name: 'write_canvas',
    description: 'Write/replace all code in the editor. Use only for completely new code.',
    parameters: {
      type: 'object',
      properties: {
        code: { type: 'string', description: 'The code to write' },
        language: { type: 'string', description: 'Programming language' },
        filename: { type: 'string', description: 'Optional filename' }
      },
      required: ['code']
    }
  }
];

// =============================================================================
// CHECK DIAGNOSTICS
// =============================================================================

function checkDiagnosticsTool() {
  // Check if Monaco is available
  if (typeof monaco === 'undefined') {
    return { success: false, error: 'Monaco editor not loaded' };
  }
  
  // Get the editor instance from canvas
  if (typeof window.monacoEditor === 'undefined' && typeof monacoEditor === 'undefined') {
    // Try to get from global scope via canvas.js
    const editor = window.Canvas?.getEditor?.() || window.monacoEditor;
    if (!editor) {
      return { success: false, error: 'No active editor' };
    }
  }
  
  // Get model from editor
  const editor = window.monacoEditor || monacoEditor;
  if (!editor) {
    return { success: false, error: 'Editor not available' };
  }
  
  const model = editor.getModel();
  if (!model) {
    return { success: false, error: 'No file open in editor' };
  }
  
  // Get all markers (diagnostics) for this model
  const markers = monaco.editor.getModelMarkers({ resource: model.uri });
  
  // Format diagnostics
  const diagnostics = {
    errors: [],
    warnings: [],
    infos: [],
    total: markers.length
  };
  
  markers.forEach(marker => {
    const issue = {
      line: marker.startLineNumber,
      column: marker.startColumn,
      endLine: marker.endLineNumber,
      endColumn: marker.endColumn,
      message: marker.message,
      source: marker.source || 'unknown'
    };
    
    if (marker.severity === monaco.MarkerSeverity.Error) {
      diagnostics.errors.push(issue);
    } else if (marker.severity === monaco.MarkerSeverity.Warning) {
      diagnostics.warnings.push(issue);
    } else if (marker.severity === monaco.MarkerSeverity.Info) {
      diagnostics.infos.push(issue);
    }
  });
  
  // Get current file info
  const tab = window.canvasTabs?.find(t => t.id === window.activeTabId);
  
  return {
    success: true,
    filename: tab?.name || 'unknown',
    language: tab?.language || 'unknown',
    diagnostics: diagnostics,
    summary: `${diagnostics.errors.length} errors, ${diagnostics.warnings.length} warnings`
  };
}

// =============================================================================
// CANVAS READ
// =============================================================================

function readCanvasTool() {
  // Use the Canvas API from canvas.js
  if (typeof window.Canvas !== 'undefined' && window.Canvas.read) {
    return window.Canvas.read();
  }
  
  return { success: false, error: 'Canvas not available' };
}

// =============================================================================
// CANVAS WRITE
// =============================================================================

function writeCanvasTool(code, language, filename) {
  if (typeof window.Canvas !== 'undefined' && window.Canvas.write) {
    return window.Canvas.write(code, language, filename);
  }
  
  return { success: false, error: 'Canvas not available' };
}

// =============================================================================
// CANVAS EDIT
// =============================================================================

function editCanvasTool(startLine, endLine, newCode) {
  console.log('[Tools] editCanvas called:', { startLine, endLine, newCodeLength: newCode?.length, newCodePreview: newCode?.substring(0, 100) });
  
  if (typeof window.Canvas !== 'undefined' && window.Canvas.edit) {
    return window.Canvas.edit(startLine, endLine, newCode);
  }
  
  return { success: false, error: 'Canvas not available' };
}

// =============================================================================
// TOOL EXECUTOR
// =============================================================================

function executeTool(name, args = {}) {
  console.log('[Tools] Executing:', name, args);
  
  switch (name) {
    case 'start_task':
      return startAgentLoop(args.task);
    
    case 'end_task':
      return endAgentLoop(args.summary);
    
    case 'check_diagnostics':
      return checkDiagnosticsTool();
    
    case 'read_canvas':
      return readCanvasTool();
    
    case 'write_canvas':
      return writeCanvasTool(args.code, args.language, args.filename);
    
    case 'edit_canvas':
      return editCanvasTool(args.start_line, args.end_line, args.new_code);
    
    default:
      return { success: false, error: `Unknown tool: ${name}` };
  }
}

// =============================================================================
// EXPORTS
// =============================================================================

window.AITools = {
  definitions: TOOL_DEFINITIONS,
  execute: executeTool,
  isAgentLoopActive: isAgentLoopActive,
  checkDiagnostics: checkDiagnosticsTool,
  readCanvas: readCanvasTool,
  writeCanvas: writeCanvasTool,
  editCanvas: editCanvasTool
};
