(() => {
  const q = selector => document.querySelector(selector);
  const tabs = [...document.querySelectorAll('[role=tab]')];
  function selectTab(tab) {
    tabs.forEach(item => {
      const active = item === tab;
      item.setAttribute('aria-selected', String(active));
      item.tabIndex = active ? 0 : -1;
      document.getElementById(item.getAttribute('aria-controls')).hidden = !active;
    });
  }
  tabs.forEach((tab, index) => {
    tab.addEventListener('click', () => selectTab(tab));
    tab.addEventListener('keydown', event => {
      let target;
      if (event.key === 'ArrowRight') target = tabs[(index + 1) % tabs.length];
      if (event.key === 'ArrowLeft') target = tabs[(index + tabs.length - 1) % tabs.length];
      if (event.key === 'Home') target = tabs[0];
      if (event.key === 'End') target = tabs[tabs.length - 1];
      if (target) { event.preventDefault(); selectTab(target); target.focus(); }
    });
  });
  q('#mixed-checkbox').indeterminate = true;
  document.querySelectorAll('[data-notice]').forEach(button => button.addEventListener('click', () => q('#component-notice').textContent = button.dataset.notice));

  const shows = {
    orbital: {title:'Orbital',year:2026,seasons:{1:['Arrival','First contact','The signal','Homeward','A new orbit'],2:['Return','The beacon','Far side','Passage','Landing']}},
    night: {title:'Night Train',year:2026,seasons:{1:['Platform one','Departure','The passenger','Midnight','Terminus'],2:['New timetable','Border crossing','The letter','Nightfall','Last stop']}}
  };
  const templates = {full:'{series} ({year}) - S{season}E{episode} - {title}.mkv',compact:'{series} ({year}) - S{season}E{episode}.mkv'};
  const oldNames = ['Orbital.01.1080p.mkv','Orbital.02.1080p.mkv','Orbital.finale.mkv'];
  const escape = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const pad = n => String(n).padStart(2,'0');
  const dialog = q('#file-dialog');
  let mode = 'import', rows = [], draft = null, trigger = null;
  let format = 'full';
  function nameFor(row, episode = row.episode) {
    if (!episode) return '';
    const show = shows[row.series];
    const tokens = {series:show.title,year:show.year,season:pad(row.season),episode:pad(episode),title:show.seasons[row.season][episode-1]};
    return templates[format].replace(/\{(\w+)\}/g, (_, key) => tokens[key]);
  }
  const selected = () => rows.map((row,index) => row.selected && !row.done ? index : -1).filter(index => index >= 0);
  function notify(text, error = false) {
    q('#dialog-feedback').textContent = text;
    q('#dialog-feedback').className = 'mt-3 text-caption ' + (error ? 'text-danger' : 'text-muted');
    q('#dialog-feedback').setAttribute('role', error ? 'alert' : 'status');
  }
  function cancelDraft() { draft = null; q('#order-apply').hidden = true; }
  function detect(row) {
    const match = row.old.match(/\.(\d{2})\./);
    row.episode = match && Number(match[1]) <= shows[row.series].seasons[row.season].length ? Number(match[1]) : null;
  }
  function render() {
    const importing = mode === 'import', ids = selected(), available = rows.filter(row => !row.done);
    const unresolved = available.filter(row => !row.episode).length;
    q('#dialog-title').textContent = importing ? 'Review import' : 'Rename existing files';
    q('#dialog-description').textContent = `${available.length} files · ${importing ? (unresolved ? unresolved + ' needs a match' : 'All matched') : 'Names generated from saved rules'}`;
    q('#bulk-tools').hidden = !importing;
    q('#rename-format').hidden = importing;
    q('#rename-format').textContent = 'Orbital · Season 1 · ' + templates[format];
    q('#selected-count').textContent = `${ids.length} selected`;
    q('#operation-note').textContent = importing ? 'Hard-link · Keep seeding' : 'Existing episode matches retained';
    q('#commit-files').textContent = `${importing ? 'Import' : 'Rename'} ${ids.length} ${ids.length === 1 ? 'file' : 'files'}`;
    q('#commit-files').disabled = !ids.length || Boolean(draft);
    q('#select-all').checked = available.length > 0 && ids.length === available.length;
    q('#select-all').indeterminate = ids.length > 0 && ids.length < available.length;
    q('#select-all').disabled = !available.length;
    document.querySelectorAll('#bulk-tools button,#bulk-tools select').forEach(el => el.disabled = !ids.length);
    for (const field of ['series','season']) {
      const select = q('#bulk-' + field), values = new Set(ids.map(i => String(rows[i][field])));
      select.querySelector('[data-mixed]')?.remove();
      if (values.size > 1) {
        const option = document.createElement('option'); option.value = ''; option.textContent = 'Mixed'; option.disabled = true; option.dataset.mixed = '';
        select.prepend(option); select.value = '';
      } else if (ids.length) select.value = String(rows[ids[0]][field]);
    }
    q('#file-rows').innerHTML = rows.map((row,index) => {
      if (row.done) return '';
      const episode = draft?.find(item => item.index === index)?.episode ?? row.episode;
      const select = `<select class="yk-select font-mono text-caption ${episode ? '' : 'border-warning'}" data-match="${index}" aria-label="New name for ${escape(row.old)}" ${draft?'disabled':''}><option value="">Choose the correct name…</option>${shows[row.series].seasons[row.season].map((_,i) => `<option value="${i+1}" ${episode===i+1?'selected':''}>${escape(nameFor(row,i+1))}</option>`).join('')}</select>`;
      return `<div class="yk-modal-row"><label class="yk-check-target flex min-w-0 items-center gap-2"><input class="yk-checkbox" type="checkbox" data-row="${index}" ${row.selected?'checked':''} aria-label="Select ${escape(row.old)}"><span class="yk-code break-all">${escape(row.old)}</span></label><span class="text-muted" aria-hidden="true">→</span>${importing?select:`<span class="yk-code break-all">${escape(nameFor(row))}</span>`}</div>`;
    }).join('') || '<p class="py-6 text-muted">No remaining files in this demo batch.</p>';
    document.querySelectorAll('[data-row]').forEach(el => el.addEventListener('change', () => {
      const index = Number(el.dataset.row); rows[index].selected = el.checked; cancelDraft(); notify(''); render(); q(`[data-row="${index}"]`)?.focus();
    }));
    document.querySelectorAll('[data-match]').forEach(el => el.addEventListener('change', () => {
      const index = Number(el.dataset.match); rows[index].episode = el.value ? Number(el.value) : null; cancelDraft(); notify(''); render(); q(`[data-match="${index}"]`)?.focus();
    }));
  }
  document.querySelectorAll('[data-open]').forEach(button => button.addEventListener('click', () => {
    trigger = button; mode = button.dataset.open;
    rows = oldNames.map((old,index) => ({old,series:'orbital',season:1,episode:mode==='rename'||index<2?index+1:null,selected:true,done:false}));
    cancelDraft(); q('#order-tools').hidden = true; notify(''); render(); dialog.showModal();
  }));
  document.querySelectorAll('[data-close]').forEach(button => button.addEventListener('click', () => dialog.close()));
  dialog.addEventListener('close', () => trigger?.focus());
  q('#select-all').addEventListener('change', event => { rows.forEach(row => {if(!row.done) row.selected=event.target.checked}); cancelDraft(); render(); });
  for (const field of ['series','season']) q('#bulk-'+field).addEventListener('change', event => {
    selected().forEach(index => {rows[index][field] = field==='season'?Number(event.target.value):event.target.value;detect(rows[index])});
    cancelDraft(); notify('Updated selected files and refreshed their matches.'); render();
  });
  q('#redetect').addEventListener('click', () => {selected().forEach(index => detect(rows[index]));cancelDraft();notify('Matches refreshed. Uncertain files still need review.');render()});
  q('#order-open').addEventListener('click', () => {q('#order-tools').hidden=false;q('#order-start').focus()});
  q('#order-cancel').addEventListener('click', () => {cancelDraft();q('#order-tools').hidden=true;notify('');render();q('#order-open').focus()});
  q('#order-start').addEventListener('change', () => {cancelDraft();render()});
  q('#order-preview').addEventListener('click', () => {
    const ids = selected().sort((a,b) => rows[a].old.localeCompare(rows[b].old,undefined,{numeric:true}));
    if (!ids.length) return notify('Select at least one file.',true);
    if(new Set(ids.map(i=>rows[i].series+'/'+rows[i].season)).size>1)return notify('Ordered assignment needs one series and season.',true);
    const start = Number(q('#order-start').value), first = rows[ids[0]];
    if(start+ids.length-1>shows[first.series].seasons[first.season].length)return notify('Not enough episodes after this starting point.',true);
    draft = ids.map((index,i)=>({index,episode:start+i}));q('#order-apply').hidden=false;
    notify('Preview only · Sorted by filename. Check the proposed names, then apply assignments.');render();
  });
  q('#order-apply').addEventListener('click', () => {draft?.forEach(({index,episode})=>rows[index].episode=episode);cancelDraft();q('#order-tools').hidden=true;notify('Assignments applied to this preview.');render();q('#order-open').focus()});
  q('#commit-files').addEventListener('click', () => {
    const ids = selected();
    if(ids.some(i=>!rows[i].episode))return notify('Choose a name for each selected file, or uncheck unresolved files.',true);
    const targets = ids.map(i=>nameFor(rows[i]));
    if(new Set(targets).size!==targets.length)return notify('Two selected files have the same destination. Correct the match or uncheck one.',true);
    if(targets.some(name=>rows.some(row=>row.done&&nameFor(row)===name)))return notify('This destination was already used in the demo. Correct the match or skip that file.',true);
    ids.forEach(i=>rows[i].done=true);render();notify(`Demo complete: ${ids.length} files would be ${mode==='import'?'imported':'renamed'}. No real files changed.`);q('[data-close]').focus();
  });
  function namingExample(){q('#naming-example').textContent=nameFor({series:'orbital',season:1,episode:1})}
  q('#naming-format').addEventListener('change',event=>{format=event.target.value;namingExample()});
  namingExample();
})();
