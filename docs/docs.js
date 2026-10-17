// Rail Documentation Common Interactive Logic
document.addEventListener('DOMContentLoaded', () => {
  initHamburger();
  initCursor();
  initCodeCopy();
  initSearch();
  initVirtualRibbon();
  initProfileLab();
  initKeyboardTester();
});

// 1. Mobile Hamburger Menu
function initHamburger() {
  const btn = document.querySelector('.hamburger');
  const sidebar = document.querySelector('aside.docs-sidebar');
  const overlay = document.querySelector('.sidebar-overlay');

  if (!btn || !sidebar) return;

  function toggleMenu(open) {
    const isOpen = open !== undefined ? open : !sidebar.classList.contains('open');
    sidebar.classList.toggle('open', isOpen);
    if (overlay) overlay.classList.toggle('open', isOpen);
    btn.setAttribute('aria-expanded', isOpen);
  }

  btn.addEventListener('click', () => toggleMenu());
  if (overlay) overlay.addEventListener('click', () => toggleMenu(false));

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && sidebar.classList.contains('open')) {
      toggleMenu(false);
    }
  });
}

// 2. Custom Cursor
function initCursor() {
  const cur = document.getElementById('cur');
  if (!cur) return;

  if (matchMedia('(pointer:fine)').matches) {
    let mx = 0, my = 0, cx = 0, cy = 0;
    addEventListener('pointermove', (e) => {
      mx = e.clientX;
      my = e.clientY;
      const isInteractive = e.target.closest('button, a, select, input, .card, .v-btn, .v-window, .kb-item');
      cur.style.width = cur.style.height = isInteractive ? '32px' : '18px';
      cur.style.borderColor = isInteractive ? '#fff' : 'var(--blue)';
    });

    (function loop() {
      cx += (mx - cx) * 0.2;
      cy += (my - cy) * 0.2;
      cur.style.left = cx + 'px';
      cur.style.top = cy + 'px';
      requestAnimationFrame(loop);
    })();
  } else {
    cur.style.display = 'none';
  }
}

// 3. Code Copy functionality
function initCodeCopy() {
  document.querySelectorAll('.btn-copy').forEach(btn => {
    btn.addEventListener('click', () => {
      const targetId = btn.getAttribute('data-target');
      const codeEl = targetId ? document.getElementById(targetId) : btn.closest('.code-block')?.querySelector('code, pre');
      const text = codeEl ? codeEl.innerText.trim() : btn.getAttribute('data-code');

      if (text) {
        navigator.clipboard.writeText(text).then(() => {
          const original = btn.textContent;
          btn.textContent = (typeof docT === 'function' ? docT('lbl_copied') : 'Copied!');
          btn.style.color = '#fff';
          setTimeout(() => {
            btn.textContent = original;
            btn.style.color = '';
          }, 1400);
        });
      }
    });
  });
}

// 4. Quick Search / Filter
function initSearch() {
  const searchInput = document.querySelector('.search-box input');
  if (!searchInput) return;

  document.addEventListener('keydown', (e) => {
    if (e.key === '/' && document.activeElement.tagName !== 'INPUT' && document.activeElement.tagName !== 'TEXTAREA') {
      e.preventDefault();
      searchInput.focus();
    }
  });

  searchInput.addEventListener('input', (e) => {
    const query = e.target.value.toLowerCase().trim();
    const navItems = document.querySelectorAll('.nav-item');
    navItems.forEach(item => {
      const text = item.textContent.toLowerCase();
      item.style.display = text.includes(query) ? '' : 'none';
    });
  });
}

// 5. Virtual 2D Ribbon Simulator
function initVirtualRibbon() {
  const track = document.getElementById('vTrack');
  if (!track) return;

  let columns = [
    { id: 1, title: 'GitHub', windows: ['github.com/mrdevs2011/rail'] },
    { id: 2, title: 'Docs', windows: ['rail-browser.vercel.app/docs', 'api/reference'] },
    { id: 3, title: 'Terminal', windows: ['cargo run --release'] }
  ];
  let curCol = 0;
  let curRow = 0;
  let isOverview = false;

  function render() {
    track.innerHTML = '';
    columns.forEach((col, cIdx) => {
      const colEl = document.createElement('div');
      colEl.className = 'v-column' + (cIdx === curCol ? ' focused' : '');
      colEl.style.transform = isOverview ? 'scale(0.85)' : '';

      const headerEl = document.createElement('div');
      headerEl.className = 'v-col-header';
      headerEl.innerHTML = `<span>Col ${cIdx + 1}: ${col.title}</span><span style="color:#666">x${col.windows.length}</span>`;
      colEl.appendChild(headerEl);

      const stackEl = document.createElement('div');
      stackEl.className = 'v-stack-container';

      col.windows.forEach((win, rIdx) => {
        const winEl = document.createElement('div');
        winEl.className = 'v-window' + (cIdx === curCol && rIdx === curRow ? ' focused' : '');
        winEl.innerHTML = `<b style="margin-bottom:4px">${win}</b><span style="font-size:10px;color:#777">Ctrl+V stak #${rIdx + 1}</span>`;
        winEl.addEventListener('click', () => {
          curCol = cIdx;
          curRow = rIdx;
          render();
        });
        stackEl.appendChild(winEl);
      });

      colEl.appendChild(stackEl);
      track.appendChild(colEl);
    });

    // Update status
    const statEl = document.getElementById('vStatus');
    if (statEl) {
      statEl.innerHTML = `Fokus: <b>Ustun ${curCol + 1} / ${columns.length}</b>, <b>Oyna ${curRow + 1} / ${columns[curCol]?.windows.length || 1}</b> | Zoom: ${isOverview ? '65% (Overview)' : '100%'}`;
    }

    // Scroll active into view
    const activeColEl = track.children[curCol];
    if (activeColEl) {
      activeColEl.scrollIntoView({ behavior: 'smooth', inline: 'center', block: 'nearest' });
    }
  }

  // Simulator Buttons
  const btnAddCol = document.getElementById('vAddCol');
  if (btnAddCol) {
    btnAddCol.onclick = () => {
      columns.push({
        id: Date.now(),
        title: `Ustun ${columns.length + 1}`,
        windows: [`webpage_${columns.length + 1}.html`]
      });
      curCol = columns.length - 1;
      curRow = 0;
      render();
    };
  }

  const btnAddStack = document.getElementById('vAddStack');
  if (btnAddStack) {
    btnAddStack.onclick = () => {
      if (!columns[curCol]) return;
      columns[curCol].windows.push(`split_view_${columns[curCol].windows.length + 1}.html`);
      curRow = columns[curCol].windows.length - 1;
      render();
    };
  }

  const btnClose = document.getElementById('vClose');
  if (btnClose) {
    btnClose.onclick = () => {
      if (columns.length === 0) return;
      if (columns[curCol].windows.length > 1) {
        columns[curCol].windows.splice(curRow, 1);
        curRow = Math.max(0, curRow - 1);
      } else {
        columns.splice(curCol, 1);
        curCol = Math.max(0, curCol - 1);
        curRow = 0;
      }
      if (columns.length === 0) {
        columns = [{ id: 1, title: 'Bosh sahifa', windows: ['rail://newtab'] }];
        curCol = 0;
        curRow = 0;
      }
      render();
    };
  }

  const btnOverview = document.getElementById('vOverview');
  if (btnOverview) {
    btnOverview.onclick = () => {
      isOverview = !isOverview;
      btnOverview.classList.toggle('primary', isOverview);
      render();
    };
  }

  // Keyboard controls for simulator when focused
  window.addEventListener('keydown', (e) => {
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA') return;

    if (e.ctrlKey && (e.key === 'h' || e.key === 'ArrowLeft')) {
      e.preventDefault();
      curCol = Math.max(0, curCol - 1);
      curRow = 0;
      render();
    } else if (e.ctrlKey && (e.key === 'l' || e.key === 'ArrowRight')) {
      e.preventDefault();
      curCol = Math.min(columns.length - 1, curCol + 1);
      curRow = 0;
      render();
    } else if (e.ctrlKey && (e.key === 'j' || e.key === 'ArrowDown')) {
      e.preventDefault();
      if (columns[curCol]) curRow = Math.min(columns[curCol].windows.length - 1, curRow + 1);
      render();
    } else if (e.ctrlKey && (e.key === 'k' || e.key === 'ArrowUp')) {
      e.preventDefault();
      curRow = Math.max(0, curRow - 1);
      render();
    }
  });

  render();
}

// 6. Virtual Profile Isolation Lab
function initProfileLab() {
  const p1Btn = document.getElementById('labP1');
  const p2Btn = document.getElementById('labP2');
  const loginBtn = document.getElementById('labLogin');
  const wipeBtn = document.getElementById('labWipe');
  const jar0 = document.getElementById('labJar0');
  const jar1 = document.getElementById('labJar1');
  const labMsg = document.getElementById('labMsg');

  if (!jar0 || !jar1) return;

  const profileData = [
    { name: 'Shaxsiy (Personal)', cookies: ['session_id=usr_auth_8829', 'theme=dark'], user: 'muhamad_dev' },
    { name: 'Ishchi (Work)', cookies: [], user: null }
  ];
  let activeProfile = 0;

  function renderLab() {
    [0, 1].forEach(i => {
      const jarEl = i === 0 ? jar0 : jar1;
      const cardEl = jarEl.closest('.lab-card');
      const isActive = i === activeProfile;

      if (cardEl) cardEl.classList.toggle('active', isActive);

      const items = profileData[i].cookies.length > 0
        ? profileData[i].cookies.map(c => `<div>• <code>${c}</code></div>`).join('')
        : '<span style="color:#666">Cookies: bo\'sh (no session)</span>';

      jarEl.innerHTML = `
        <div style="margin-bottom:6px">Foydalanuvchi: <b>${profileData[i].user || '(kirmagan)'}</b></div>
        <div>${items}</div>
      `;
    });

    if (p1Btn) {
      p1Btn.classList.toggle('primary', activeProfile === 0);
    }
    if (p2Btn) {
      p2Btn.classList.toggle('primary', activeProfile === 1);
    }
  }

  if (p1Btn) p1Btn.onclick = () => { activeProfile = 0; renderLab(); };
  if (p2Btn) p2Btn.onclick = () => { activeProfile = 1; renderLab(); };

  if (loginBtn) {
    loginBtn.onclick = () => {
      const p = profileData[activeProfile];
      const randToken = Math.random().toString(36).substring(2, 9);
      p.cookies = [`auth_token=tok_${randToken}`, 'user_pref=profile_' + (activeProfile + 1)];
      p.user = activeProfile === 0 ? 'personal_acc@gmail.com' : 'enterprise_dev@work.corp';
      if (labMsg) {
        labMsg.textContent = `${activeProfile === 0 ? 'Profil 1' : 'Profil 2'} tizimiga kirildi. Boshqa profil kukilari tegilmadi!`;
      }
      renderLab();
    };
  }

  if (wipeBtn) {
    wipeBtn.onclick = () => {
      profileData[activeProfile].cookies = [];
      profileData[activeProfile].user = null;
      if (labMsg) {
        labMsg.textContent = `${activeProfile === 0 ? 'Profil 1' : 'Profil 2'} kukilari tozalandi.`;
      }
      renderLab();
    };
  }

  renderLab();
}

// 7. Interactive Keyboard Tester
function initKeyboardTester() {
  const input = document.getElementById('kbTesterInput');
  const output = document.getElementById('kbTesterOutput');
  if (!input || !output) return;

  const keyMap = {
    'ctrl+t': 'Omnibar ochildi (yangi tab)',
    'ctrl+l': 'Manzil qatori (Omnibar) ochildi',
    'alt+shift+v': 'Yangi ish maydoni ochildi',
    'ctrl+w': 'Fokusdagi tab yopildi',
    'ctrl+r': 'Sahifa qayta yuklandi (Reload)',
    'alt+f': 'To\'liq eni / Standart eni rejimi',
    'alt+h': 'Chap ustunga fokus o\'tdi',
    'alt+l': 'O\'ng ustunga fokus o\'tdi',
    'alt+j': 'Ustun ichida pastdagi oynaga fokus o\'tdi',
    'alt+k': 'Ustun ichida yuqoridagi oynaga fokus o\'tdi',
    'alt+[': 'Oyna chap ustunga yig\'ildi / chapga ajratildi',
    'alt+]': 'Oyna o\'ng ustunga yig\'ildi / o\'ngga ajratildi',
    'alt+shift+h': 'Ustun chapga ko\'chdi',
    'alt+shift+l': 'Ustun o\'ngga ko\'chdi',
    'alt+shift+j': 'Oyna pastga surildi',
    'alt+shift+k': 'Oyna yuqoriga surildi',
    'ctrl+tab': 'Keyingi tabga o\'tildi',
    'ctrl+shift+tab': 'Oldingi tabga o\'tildi',
    'alt+o': 'Fazoviy Overview almashtirildi',
    'alt+b': 'Yon panel almashtirildi',
    'alt+m': 'Roadmap / Minimap almashtirildi',
    'alt+p': 'Profillar boshqaruv paneli ochildi',
    'alt+i': 'Data & Backup ochildi',
    'alt+s': 'Sozlamalar ochildi',
    'alt+shift+t': 'Touchpad kanvas rejimi almashtirildi',
    'f12': 'DevTools almashtirildi',
    'f11': 'To\'liq ekran almashtirildi',
    'alt+arrowleft': 'Orqaga',
    'alt+arrowright': 'Oldinga',
    'ctrl+q': 'Rail yopiladi'
  };

  input.addEventListener('keydown', (e) => {
    e.preventDefault();
    let combo = [];
    if (e.ctrlKey) combo.push('ctrl');
    if (e.altKey) combo.push('alt');
    if (e.shiftKey) combo.push('shift');
    if (!['Control', 'Shift', 'Alt'].includes(e.key)) {
      combo.push(e.key.toLowerCase());
    }
    const comboStr = combo.join('+');
    input.value = comboStr;

    const action = keyMap[comboStr] || `Bosilgan: ${comboStr}`;
    output.innerHTML = `<span style="color:var(--blue)">[${comboStr}]</span> → <b>${action}</b>`;

    // Highlight matching row in table if exists
    document.querySelectorAll('[data-combo]').forEach(el => {
      el.style.borderColor = el.getAttribute('data-combo') === comboStr ? 'var(--blue)' : '';
    });
  });
}

/* sidebar smooth scroll for same-page anchors */
document.addEventListener('DOMContentLoaded', function () {
  document.querySelectorAll('.docs-sidebar a[href^="#"]').forEach(function (a) {
    a.addEventListener('click', function (e) {
      var id = a.getAttribute('href').slice(1);
      var el = document.getElementById(id);
      if (!el) return;
      e.preventDefault();
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
      history.replaceState(null, '', '#' + id);
    });
  });
});
