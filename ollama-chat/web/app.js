// App State
function app() {
  return {
    count: 0,
    name: '',
    
    // Example: Call Poly backend
    async callBackend() {
      try {
        const result = await poly.invoke('greet', { name: this.name || 'World' });
        console.log('Backend says:', result);
      } catch (e) {
        console.log('Backend not available');
      }
    }
  };
}

console.log('App loaded - Edit web/ files to customize');
