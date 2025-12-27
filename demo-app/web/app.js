// App State
function app() {
  return {
    name: ''
  };
}

// Handle System Tray menu clicks
if (typeof poly !== 'undefined' && poly.tray) {
  poly.tray.onMenuClick((id) => {
    if (id === 'about') {
      poly.dialog.message('About Poly', 'Poly Demo App v0.1.0\n\nBuild native desktop apps with web technologies.', 'info');
    }
  });
}

console.log('App loaded - Edit web/ files to customize');