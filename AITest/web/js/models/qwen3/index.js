// Qwen3 Model Adapter
// Based on official Qwen3 documentation from HuggingFace
// https://huggingface.co/Qwen/Qwen3-1.7B

const Qwen3Adapter = {
  name: 'qwen3',
  
  // Qwen3 supports thinking via <think>...</think> tags
  supportsThinking: true,
  
  // Sampling parameters (from Best Practices)
  thinkingParams: {
    temperature: 0.6,
    top_p: 0.95,
    top_k: 20,
    min_p: 0
  },
  
  nonThinkingParams: {
    temperature: 0.7,
    top_p: 0.8,
    top_k: 20,
    min_p: 0
  },
  
  // Get optimal parameters based on thinking mode
  getParams(thinkingEnabled) {
    return thinkingEnabled ? this.thinkingParams : this.nonThinkingParams;
  },
  
  // Format options for Qwen3 API call
  formatOptions(options) {
    const thinkingEnabled = options.think !== false; // Default to true
    const params = this.getParams(thinkingEnabled);
    return {
      ...options,
      temperature: params.temperature,
      top_p: params.top_p,
      top_k: params.top_k,
      // Store thinking state for message formatting
      _thinkingEnabled: thinkingEnabled
    };
  },
  
  // Format messages for Qwen3 - adds /think or /no_think to last user message
  formatMessages(messages, thinkingEnabled) {
    if (!messages || messages.length === 0) return messages;
    
    console.log('[Qwen3] formatMessages called, thinkingEnabled:', thinkingEnabled);
    
    // Clone messages to avoid mutation
    const formatted = messages.map(m => ({ ...m }));
    
    // Find last user message
    for (let i = formatted.length - 1; i >= 0; i--) {
      if (formatted[i].role === 'user') {
        // Remove any existing /think or /no_think tags (including duplicates)
        let content = formatted[i].content.replace(/\s*\/(think|no_think)(\s*\/(think|no_think))*/g, '').trim();
        
        // Add the appropriate tag (only once)
        if (thinkingEnabled) {
          content += ' /think';
        } else {
          content += ' /no_think';
        }
        
        console.log('[Qwen3] Modified user message:', content);
        formatted[i] = { ...formatted[i], content };
        break;
      }
    }
    
    return formatted;
  },
  
  // Parse thinking from Qwen3 response
  // Qwen3 wraps thinking in <think>...</think> tags
  parseThinking(content, thinkingEnabled = true) {
    if (!content) return { thinking: null, content: '' };
    
    // Match <think>...</think> block
    const thinkMatch = content.match(/<think>([\s\S]*?)<\/think>/);
    
    if (thinkMatch) {
      const thinkContent = thinkMatch[1].trim();
      // Remove the think block from content
      const cleanContent = content.replace(/<think>[\s\S]*?<\/think>/, '').trim();
      
      // If thinking is disabled, don't return the thinking content
      // (Qwen3 may still output empty or minimal think blocks with /no_think)
      if (!thinkingEnabled || thinkContent.length === 0) {
        return { thinking: null, content: cleanContent };
      }
      
      return { thinking: thinkContent, content: cleanContent };
    }
    
    // No thinking block found
    return { thinking: null, content };
  },
  
  // Check if response contains thinking
  hasThinking(content) {
    return content && content.includes('<think>');
  },
  
  // Check if thinking block is complete
  isThinkingComplete(content) {
    return content && content.includes('</think>');
  },
  
  // Extract partial thinking (for streaming)
  extractPartialThinking(content) {
    if (!content) return { thinking: '', mainContent: '', isThinkingDone: false };
    
    const thinkStart = content.indexOf('<think>');
    const thinkEnd = content.indexOf('</think>');
    
    if (thinkStart === -1) {
      // No thinking block
      return { thinking: '', mainContent: content, isThinkingDone: true };
    }
    
    if (thinkEnd === -1) {
      // Thinking in progress
      const thinking = content.substring(thinkStart + 7); // After <think>
      return { thinking, mainContent: '', isThinkingDone: false };
    }
    
    // Thinking complete
    const thinking = content.substring(thinkStart + 7, thinkEnd);
    const mainContent = content.substring(thinkEnd + 8).trim(); // After </think>
    return { thinking, mainContent, isThinkingDone: true };
  },
  
  // Format tools for Qwen3 (OpenAI-compatible format)
  formatTools(tools) {
    if (!tools || tools.length === 0) return undefined;
    
    return tools.map(tool => ({
      type: 'function',
      function: {
        name: tool.name,
        description: tool.description,
        parameters: tool.parameters || { type: 'object', properties: {}, required: [] }
      }
    }));
  },
  
  // Get available tools from AITools
  getAvailableTools() {
    if (window.AITools && window.AITools.definitions) {
      return window.AITools.definitions;
    }
    return [];
  },
  
  // Build system prompt with tool descriptions
  buildToolSystemPrompt(tools) {
    if (!tools || tools.length === 0) return '';
    
    return `
You have tools. Code is in the editor - never ask for code.

OUTPUT FORMAT: Write tool calls as plain JSON text (not function calls). Example:
\`\`\`
{"tool": "check_diagnostics", "arguments": {}}
\`\`\`

AVAILABLE TOOLS:
- check_diagnostics: Check code for errors
- edit_canvas: Edit lines - {"tool": "edit_canvas", "arguments": {"start_line": 5, "end_line": 5, "new_code": "fixed"}}
- start_task: Start multi-step work - {"tool": "start_task", "arguments": {"task": "Fixing errors"}}
- end_task: Finish work - {"tool": "end_task", "arguments": {"summary": "Fixed 3 errors"}}

WORKFLOW:
1. For "check code": Use check_diagnostics, then explain findings
2. For "fix errors": 
   - start_task (once)
   - edit_canvas for each fix (explain what you fixed after each)
   - end_task with summary

Write a message after each tool call explaining what you did.`;
  },
  
  // Parse tool calls from Qwen3 response
  parseToolCalls(content) {
    const toolCalls = [];
    
    if (!content) return toolCalls;
    
    // Check for Ollama control tokens (tool call markers)
    // <ctrl61> might be tool call start, <ctrl63> might be tool call end
    const hasCtrlTokens = content.includes('<ctrl');
    if (hasCtrlTokens) {
      console.log('[Qwen3] Detected control tokens in response');
      // Try to extract content between ctrl tokens
      const ctrlMatch = content.match(/<ctrl\d+>([\s\S]*?)<ctrl\d+>/);
      if (ctrlMatch && ctrlMatch[1]) {
        const innerContent = ctrlMatch[1].trim();
        console.log('[Qwen3] Content between ctrl tokens:', innerContent);
        if (innerContent.startsWith('{')) {
          try {
            const obj = JSON.parse(innerContent);
            if (obj.tool) {
              toolCalls.push({ name: obj.tool, arguments: obj.arguments || {} });
              console.log('[Qwen3] Parsed tool from ctrl tokens:', obj.tool);
              return toolCalls;
            }
          } catch (e) {
            console.log('[Qwen3] Failed to parse ctrl token content');
          }
        }
      }
    }
    
    // Clean control characters and ctrl tokens
    const cleanContent = content
      .replace(/<ctrl\d+>/g, '')  // Remove <ctrlNN> tokens
      .replace(/[\x00-\x1F\x7F]/g, '');  // Remove control chars
    
    console.log('[Qwen3] Parsing content for tool calls:', cleanContent.substring(0, 300));
    
    // Method 1: Try standard JSON.parse on each line
    const lines = cleanContent.split('\n');
    for (const line of lines) {
      const trimmed = line.trim();
      if (trimmed.startsWith('{') && trimmed.includes('"tool"')) {
        try {
          const obj = JSON.parse(trimmed);
          if (obj.tool) {
            toolCalls.push({ name: obj.tool, arguments: obj.arguments || {} });
            console.log('[Qwen3] Parsed JSON tool call:', obj.tool, obj.arguments);
          }
        } catch (e) {
          // Try to fix common JSON issues
          let fixed = trimmed
            .replace(/,\s*}/g, '}')  // Remove trailing commas
            .replace(/'/g, '"');      // Replace single quotes
          try {
            const obj = JSON.parse(fixed);
            if (obj.tool) {
              toolCalls.push({ name: obj.tool, arguments: obj.arguments || {} });
              console.log('[Qwen3] Parsed fixed JSON:', obj.tool);
            }
          } catch (e2) {
            console.log('[Qwen3] JSON parse failed for line:', trimmed.substring(0, 100));
          }
        }
      }
    }
    
    // Method 2: Regex extraction if JSON parse failed
    if (toolCalls.length === 0) {
      // Match tool calls with arguments
      const toolRegex = /\{\s*"tool"\s*:\s*"([^"]+)"\s*,\s*"arguments"\s*:\s*(\{[^}]*\})\s*\}/g;
      let match;
      while ((match = toolRegex.exec(cleanContent)) !== null) {
        const toolName = match[1];
        let args = {};
        try {
          args = JSON.parse(match[2]);
        } catch (e) {
          // Parse arguments manually
          const argsStr = match[2];
          const argMatches = argsStr.matchAll(/"(\w+)"\s*:\s*(?:"([^"]*)"|(\d+))/g);
          for (const argMatch of argMatches) {
            const key = argMatch[1];
            const value = argMatch[2] !== undefined ? argMatch[2] : parseInt(argMatch[3]);
            args[key] = value;
          }
        }
        toolCalls.push({ name: toolName, arguments: args });
        console.log('[Qwen3] Regex parsed tool:', toolName, args);
      }
    }
    
    // Method 3: Simple tool name detection as fallback
    if (toolCalls.length === 0) {
      const knownTools = ['check_diagnostics', 'read_canvas', 'write_canvas', 'edit_canvas', 'start_task', 'end_task'];
      for (const tool of knownTools) {
        if (cleanContent.includes(`"tool": "${tool}"`) || cleanContent.includes(`"tool":"${tool}"`)) {
          toolCalls.push({ name: tool, arguments: {} });
          console.log('[Qwen3] Inferred tool from text:', tool);
          break;
        }
      }
    }
    
    console.log('[Qwen3] Total tool calls found:', toolCalls.length);
    return toolCalls;
  },
  
  // Best practice: Don't include thinking in conversation history
  cleanResponseForHistory(content) {
    return this.parseThinking(content).content;
  }
};

// Register with ModelAdapters
if (window.ModelAdapters) {
  window.ModelAdapters.register('qwen3', Qwen3Adapter);
}

window.Qwen3Adapter = Qwen3Adapter;
