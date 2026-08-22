document.addEventListener('DOMContentLoaded', () => {
  // 1. Copy to Clipboard Functionality
  const copyButtons = document.querySelectorAll('.copy-btn');
  copyButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      const targetId = btn.getAttribute('data-copy-target');
      const targetEl = document.getElementById(targetId);
      const textToCopy = targetEl ? targetEl.innerText.trim() : btn.getAttribute('data-copy-text');

      if (textToCopy) {
        navigator.clipboard.writeText(textToCopy).then(() => {
          const originalHTML = btn.innerHTML;
          btn.innerHTML = `
            <svg class="w-4 h-4 text-orange-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
            </svg>
            <span class="text-xs text-orange-400 font-medium">Copied!</span>
          `;
          setTimeout(() => {
            btn.innerHTML = originalHTML;
          }, 2000);
        }).catch(err => {
          console.error('Clipboard copy failed:', err);
        });
      }
    });
  });

  // 2. Tab Switcher for Quick Start Docs
  const tabButtons = document.querySelectorAll('.tab-btn');
  const tabContents = document.querySelectorAll('.tab-content');

  tabButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      const tabId = btn.getAttribute('data-tab');

      tabButtons.forEach(b => {
        b.classList.remove('border-orange-500', 'text-orange-400', 'bg-zinc-800/50');
        b.classList.add('border-transparent', 'text-zinc-400', 'hover:text-zinc-200');
      });

      btn.classList.remove('border-transparent', 'text-zinc-400', 'hover:text-zinc-200');
      btn.classList.add('border-orange-500', 'text-orange-400', 'bg-zinc-800/50');

      tabContents.forEach(content => {
        if (content.id === tabId) {
          content.classList.add('active');
        } else {
          content.classList.remove('active');
        }
      });
    });
  });

  // 3. Live eBPF Terminal Event Stream Simulation
  const terminalLogs = document.getElementById('terminal-logs');
  if (terminalLogs) {
    const sampleProcesses = [
      { pid: 1420, comm: 'python3 ./train.py' },
      { pid: 2189, comm: 'git commit -m "feat"' },
      { pid: 3042, comm: 'cargo build --release' },
      { pid: 4891, comm: 'node server.js' },
      { pid: 5120, comm: 'docker-containerd' },
      { pid: 6301, comm: 'nginx: worker process' },
      { pid: 7412, comm: 'rg --files-with-matches' },
      { pid: 8904, comm: 'htop' },
      { pid: 9112, comm: 'rustc src/main.rs' },
      { pid: 9840, comm: 'curl -fsSL https://...' },
    ];

    let count = 0;

    function addTerminalEvent() {
      const proc = sampleProcesses[Math.floor(Math.random() * sampleProcesses.length)];
      const isExec = Math.random() > 0.35;
      const eventType = isExec ? 'EXEC' : 'EXIT';
      const eventClass = isExec ? 'text-orange-400 font-semibold' : 'text-rose-400 font-semibold';
      const timestamp = new Date().toISOString().substring(11, 19);

      const line = document.createElement('div');
      line.className = 'flex items-center space-x-3 py-1 font-mono text-xs hover:bg-zinc-900/60 px-2 rounded transition-colors';
      line.innerHTML = `
        <span class="text-zinc-500">${timestamp}</span>
        <span class="${eventClass} px-1.5 py-0.5 rounded bg-zinc-900 border ${isExec ? 'border-orange-500/30' : 'border-rose-500/30'}">${eventType}</span>
        <span class="text-zinc-400">pid=<span class="text-zinc-200">${proc.pid + count}</span></span>
        <span class="text-zinc-300 truncate">comm=<span class="text-orange-300">"${proc.comm}"</span></span>
      `;

      terminalLogs.appendChild(line);
      count++;

      // Limit max terminal history rows
      if (terminalLogs.children.length > 25) {
        terminalLogs.removeChild(terminalLogs.firstChild);
      }

      // Auto scroll to bottom
      terminalLogs.scrollTop = terminalLogs.scrollHeight;
    }

    // Initial population
    for (let i = 0; i < 6; i++) {
      addTerminalEvent();
    }

    // Continuously append new events
    setInterval(addTerminalEvent, 2200);
  }
});
