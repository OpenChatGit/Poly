function app() {
  return {
    selectedFile: null,
    savedPath: null,
    selectedFolder: null,
    confirmResult: null,
    customResult: null,
    files: [],

    async openFile() {
      try {
        const path = await poly.dialog.open({ title: 'Select a file' });
        this.selectedFile = path || 'Cancelled';
      } catch (e) {
        console.error(e);
        this.selectedFile = 'Error: ' + e.message;
      }
    },

    async saveFile() {
      try {
        const path = await poly.dialog.save({ 
          title: 'Save file as',
          defaultName: 'document.txt'
        });
        this.savedPath = path || 'Cancelled';
      } catch (e) {
        console.error(e);
        this.savedPath = 'Error: ' + e.message;
      }
    },

    async pickFolder() {
      try {
        const path = await poly.dialog.folder({ title: 'Select a folder' });
        this.selectedFolder = path || 'Cancelled';
      } catch (e) {
        console.error(e);
        this.selectedFolder = 'Error: ' + e.message;
      }
    },

    async showMessage(level) {
      const titles = {
        info: 'Information',
        warning: 'Warning', 
        error: 'Error'
      };
      const messages = {
        info: 'This is a custom in-app info dialog!',
        warning: 'This is a warning - be careful!',
        error: 'Something went wrong!'
      };
      await poly.dialog.message(titles[level], messages[level], level);
    },

    async confirmAction() {
      this.confirmResult = await poly.dialog.confirm(
        'Confirm Action',
        'Do you want to proceed with this action?'
      );
    },

    async customDialog() {
      this.customResult = await poly.dialog.custom({
        type: 'info',
        title: 'Choose an Option',
        message: 'This dialog has custom buttons. Pick one!',
        buttons: [
          { text: 'Option A', value: 'A' },
          { text: 'Option B', value: 'B' },
          { text: 'Option C', value: 'C', primary: true }
        ]
      });
    },

    async readCurrentDir() {
      try {
        this.files = await poly.fs.readDir('.');
        this.$nextTick(() => lucide.createIcons());
      } catch (e) {
        console.error(e);
      }
    }
  };
}

console.log('Dialog test app loaded');
