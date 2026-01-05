// Model Adapter System
// Automatically selects the right adapter based on model name

const ModelAdapters = {
  adapters: {},
  
  // Supported model patterns (only these will be shown in UI)
  supportedPatterns: [
    { pattern: /qwen3|qwen-3|qwen_3/i, adapter: 'qwen3' }
    // Add more patterns as we add adapters:
    // { pattern: /llama/i, adapter: 'llama' },
    // { pattern: /deepseek/i, adapter: 'deepseek' },
  ],
  
  // Register a model adapter
  register(name, adapter) {
    this.adapters[name] = adapter;
    console.log(`[Models] Registered adapter: ${name}`);
  },
  
  // Check if a model is supported
  isSupported(modelName) {
    if (!modelName) return false;
    return this.supportedPatterns.some(p => p.pattern.test(modelName));
  },
  
  // Filter list of models to only supported ones
  filterSupported(modelNames) {
    return modelNames.filter(name => this.isSupported(name));
  },
  
  // Get adapter for a model name
  getAdapter(modelName) {
    if (!modelName) return null;
    
    for (const { pattern, adapter } of this.supportedPatterns) {
      if (pattern.test(modelName)) {
        return this.adapters[adapter] || null;
      }
    }
    
    return null;
  },
  
  // Get adapter name for display
  getAdapterName(modelName) {
    const adapter = this.getAdapter(modelName);
    return adapter?.name || 'unsupported';
  }
};

window.ModelAdapters = ModelAdapters;
