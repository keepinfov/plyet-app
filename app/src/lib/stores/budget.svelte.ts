import { invoke } from '@tauri-apps/api/core';
import { theme } from 'reglass-material/theme';
import type { AppData, Budget, Item, Category, Recurring, Product, ItemSource, MaterializeResult, ProductResult, UnmaterializeResult } from '$lib/types';
import { nextOccurrences, productOccurrences } from '$lib/finance';
import { currentPeriod, periodOf, periodLabel } from '$lib/dates';

type LogLevel = 'ok' | 'err' | 'warn' | 'info';

interface LogEntry {
  ts: string;
  level: LogLevel;
  msg: string;
}

/**
 * View scope within the current root budget:
 * - 'all'          — «Весь бюджет», aggregates every sub-budget
 * - 'YYYY-MM'      — one month sub-budget (plus reflected custom items)
 * - 'custom:<id>'  — one custom sub-budget
 */
type Scope = string;

const LAST_ROOT_KEY = 'plyet:lastRootId';
const LAST_SCOPE_KEY = 'plyet:lastScope';

class BudgetStore {
  data = $state<AppData | null>(null);
  loading = $state(true);
  currentRootId = $state<number>(1);
  currentScope = $state<Scope>(currentPeriod());
  currentScreen = $state<'feed' | 'regular'>('feed');
  currentTab = $state<'expenses' | 'income' | 'all'>('expenses');
  currentSort = $state<'date-desc' | 'date-asc' | 'price-desc' | 'price-asc' | 'name-asc' | 'name-desc'>('date-desc');
  searchQuery = $state('');
  categoryFilter = $state<string | null>(null);
  dateRange = $state<'all' | 'this-month' | 'last-month'>('all');
  snackbarMsg = $state('');
  snackbarShow = $state(false);
  snackbarTimer: ReturnType<typeof setTimeout> | null = null;

  logs = $state<LogEntry[]>([]);
  showLogs = $state(false);
  showDebug = $state(false);
  showBudgetModal = $state(false);
  showSettingsModal = $state(false);
  showRecurringModal = $state(false);
  editingRecurring = $state<Recurring | null>(null);
  showProductModal = $state(false);
  editingProduct = $state<Product | null>(null);
  newProductKind = $state<'deposit' | 'loan' | 'mortgage'>('deposit');

  confirmMsg = $state('');
  confirmShow = $state(false);
  confirmCallback = $state<(() => void) | null>(null);
  confirmLabel = $state('Удалить');

  // Theme state is owned by the reglass-material theme controller; these
  // delegate so existing call-sites keep working unchanged.
  get theme(): 'light' | 'dark' { return theme.mode; }
  get themePref(): 'light' | 'dark' | 'system' { return theme.pref; }
  get accentColor(): string { return theme.accent; }
  get blurEnabled(): boolean { return theme.blurEnabled; }
  get transparencyEnabled(): boolean { return theme.transparencyEnabled; }

  cycleTheme() { theme.cyclePref(); }
  setAccentColor(color: string) { theme.setAccent(color); }
  toggleBlur() { theme.toggleBlur(); }
  toggleTransparency() { theme.toggleTransparency(); }

  // ── Hierarchy selectors ─────────────────────────────────────

  roots = $derived(this.data?.budgets.filter(b => b.parent_id == null) ?? []);

  currentRoot = $derived(this.roots.find(b => b.id === this.currentRootId) ?? null);

  /** All sub-budgets of the current root. */
  childrenOfRoot = $derived(
    this.data?.budgets.filter(b => b.parent_id === this.currentRootId) ?? []
  );

  /** Month sub-budgets of the current root, newest first. */
  monthBudgets = $derived(
    this.childrenOfRoot
      .filter(b => b.kind === 'month' && b.period)
      .sort((a, b) => (b.period ?? '').localeCompare(a.period ?? ''))
  );

  customBudgets = $derived(this.childrenOfRoot.filter(b => b.kind === 'custom'));

  scopeType = $derived<'all' | 'month' | 'custom'>(
    this.currentScope === 'all' ? 'all'
      : this.currentScope.startsWith('custom:') ? 'custom'
      : 'month'
  );

  scopePeriod = $derived(this.scopeType === 'month' ? this.currentScope : null);

  scopeCustomId = $derived(
    this.scopeType === 'custom' ? Number(this.currentScope.slice('custom:'.length)) : null
  );

  currentMonthBudget = $derived(
    this.monthBudgets.find(b => b.period === this.scopePeriod) ?? null
  );

  currentCustomBudget = $derived(
    this.customBudgets.find(b => b.id === this.scopeCustomId) ?? null
  );

  /** The budget the scope maps to (root for 'all'; may be null for a month with no row yet). */
  currentBudget = $derived<Budget | null>(
    this.scopeType === 'all' ? this.currentRoot
      : this.scopeType === 'custom' ? this.currentCustomBudget
      : this.currentMonthBudget
  );

  /**
   * Real (persisted) items in the current scope. Month scope additionally
   * pulls in items of custom sub-budgets with `reflect_in_months`, tagged
   * with `reflectedFrom` so cards can show the source badge.
   */
  scopeItems = $derived.by<Item[]>(() => {
    const root = this.currentRoot;
    if (!root) return [];
    if (this.scopeType === 'custom') {
      return this.currentCustomBudget?.items ?? [];
    }
    if (this.scopeType === 'month') {
      const period = this.scopePeriod!;
      const out: Item[] = [...(this.currentMonthBudget?.items ?? [])];
      for (const custom of this.customBudgets) {
        if (!custom.reflect_in_months) continue;
        for (const item of custom.items) {
          if (periodOf(item.date) === period) out.push({ ...item, reflectedFrom: custom.name });
        }
      }
      return out;
    }
    // 'all': the whole tree of the current root
    return [...root.items, ...this.childrenOfRoot.flatMap(b => b.items)];
  });

  /** Scope limit before income: month inherits the root default until its row exists. */
  scopeLimit = $derived.by(() => {
    if (this.scopeType === 'custom') return this.currentCustomBudget?.limit ?? 0;
    if (this.scopeType === 'month') return this.currentMonthBudget?.limit ?? this.currentRoot?.limit ?? 0;
    return this.currentRoot?.limit ?? 0;
  });

  /** Limit topped up by completed income in scope (the number the TopBar shows). */
  effectiveLimit = $derived(
    this.scopeLimit
    + this.scopeItems
      .filter(i => i.item_type === 'income' && i.completed)
      .reduce((s, i) => s + i.amount, 0)
  );

  /** Recurring rules belonging to the current root budget. */
  recurringForBudget = $derived(
    this.data?.recurring.filter(r => r.budget_id === this.currentRootId) ?? []
  );

  /** Products (deposits/loans) belonging to the current root budget. */
  productsForBudget = $derived(
    this.data?.products.filter(p => p.budget_id === this.currentRootId) ?? []
  );

  /** Virtual feed items projected from active products up to each product's horizon. */
  productOccurrences = $derived.by(() => {
    const out: Item[] = [];
    for (const p of this.productsForBudget) {
      out.push(...productOccurrences(p, p.horizon));
    }
    return out;
  });

  /**
   * Virtual (not-yet-materialized) feed items projected from recurring rules up
   * to each rule's horizon. Never persisted; synthetic negative ids avoid
   * colliding with real positive item ids.
   */
  virtualOccurrences = $derived.by(() => {
    const out: Item[] = [];
    for (const r of this.recurringForBudget) {
      const dates = nextOccurrences(r, r.horizon);
      dates.forEach((date, i) => {
        const source: ItemSource = { kind: 'recurring', id: r.id, date };
        out.push({
          id: -(r.id * 100 + i) - 1,
          name: r.name,
          amount: r.amount,
          category: r.category,
          date,
          description: r.description,
          link: r.link,
          item_type: r.item_type,
          completed: false,
          virtual: true,
          source,
          uuid: '',
          created_at: '',
          updated_at: '',
          deleted_at: null,
          author_id: null,
        });
      });
    }
    return out;
  });

  /**
   * Real items plus virtual occurrences — the source for the feed/projections.
   * Month scope shows only the occurrences falling in that month; custom
   * sub-budgets have no schedules of their own, so no virtuals there.
   */
  feedItems = $derived.by(() => {
    if (this.scopeType === 'custom') return this.scopeItems;
    const virtuals = [...this.virtualOccurrences, ...this.productOccurrences];
    if (this.scopeType === 'month') {
      const period = this.scopePeriod!;
      return [...this.scopeItems, ...virtuals.filter(v => periodOf(v.date) === period)];
    }
    return [...this.scopeItems, ...virtuals];
  });

  filteredItems = $derived.by(() => {
    let items = this.feedItems;

    // Tab filter
    if (this.currentTab === 'expenses') items = items.filter(i => i.item_type !== 'income');
    else if (this.currentTab === 'income') items = items.filter(i => i.item_type === 'income');

    // Text search
    if (this.searchQuery) {
      const q = this.searchQuery.toLowerCase();
      items = items.filter(i => i.name.toLowerCase().includes(q) || i.description.toLowerCase().includes(q));
    }

    // Category filter
    if (this.categoryFilter) {
      items = items.filter(i => i.category === this.categoryFilter);
    }

    // Date range filter (meaningful in the 'all' scope; month scope is already
    // period-bounded and Tabs hides the control there)
    if (this.dateRange !== 'all' && this.scopeType === 'all') {
      const now = new Date();
      let monthStart: Date;
      if (this.dateRange === 'this-month') {
        monthStart = new Date(now.getFullYear(), now.getMonth(), 1);
      } else {
        monthStart = new Date(now.getFullYear(), now.getMonth() - 1, 1);
      }
      const nextMonth = new Date(monthStart.getFullYear(), monthStart.getMonth() + 1, 1);
      const startDate = `${monthStart.getFullYear()}-${String(monthStart.getMonth() + 1).padStart(2, '0')}-01`;
      const endDate = `${nextMonth.getFullYear()}-${String(nextMonth.getMonth() + 1).padStart(2, '0')}-01`;
      items = items.filter(i => i.date >= startDate && i.date < endDate);
    }

    // Sort
    items = [...items].sort((a, b) => {
      switch (this.currentSort) {
        case 'date-desc': return b.date.localeCompare(a.date);
        case 'date-asc': return a.date.localeCompare(b.date);
        case 'price-desc': return b.amount - a.amount;
        case 'price-asc': return a.amount - b.amount;
        case 'name-asc': return a.name.localeCompare(b.name);
        case 'name-desc': return b.name.localeCompare(a.name);
        default: return 0;
      }
    });
    return items;
  });

  addLog(level: LogLevel, msg: string) {
    this.logs = [...this.logs.slice(-199), { ts: new Date().toISOString(), level, msg }];
    // Also print to console
    const fn = level === 'err' ? console.error : level === 'warn' ? console.warn : console.log;
    fn(`[${level.toUpperCase()}]`, msg);
  }

  clearLogs() {
    this.logs = [];
  }

  copyLogs() {
    const text = this.logs.map(l => `[${l.ts}] [${l.level.toUpperCase()}] ${l.msg}`).join('\n');
    navigator.clipboard.writeText(text).catch(() => {});
  }

  private async invokeCmd<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
    const clean = JSON.parse(JSON.stringify(args));
    this.addLog('info', `${cmd} ${JSON.stringify(clean)}`);
    const result = await invoke<T>(cmd, clean);
    this.addLog('ok', `${cmd} → OK`);
    return result;
  }

  /** Every mutation returns the full live budget list — just swap it in. */
  private applyBudgets(budgets: Budget[]) {
    if (this.data) this.data.budgets = budgets;
  }

  /** Find a real item anywhere in the current data (routing spreads them across sub-budgets). */
  private findItem(itemId: number): Item | undefined {
    for (const b of this.data?.budgets ?? []) {
      const item = b.items.find(i => i.id === itemId);
      if (item) return item;
    }
    return undefined;
  }

  private persistSelection() {
    try {
      localStorage.setItem(LAST_ROOT_KEY, String(this.currentRootId));
      localStorage.setItem(LAST_SCOPE_KEY, this.currentScope);
    } catch { /* storage unavailable — selection just won't stick */ }
  }

  private restoreSelection() {
    let savedRoot = 0;
    let savedScope = '';
    try {
      savedRoot = Number(localStorage.getItem(LAST_ROOT_KEY) ?? '');
      savedScope = localStorage.getItem(LAST_SCOPE_KEY) ?? '';
    } catch { /* ignore */ }

    this.currentRootId = this.roots.some(r => r.id === savedRoot)
      ? savedRoot
      : (this.roots[0]?.id ?? 1);

    const scopeValid =
      savedScope === 'all'
      || (/^\d{4}-\d{2}$/.test(savedScope))
      || (savedScope.startsWith('custom:')
          && this.customBudgets.some(b => b.id === Number(savedScope.slice('custom:'.length))));
    this.currentScope = scopeValid ? savedScope : currentPeriod();
  }

  /** Make sure the device-local current month exists as a row (idempotent). */
  private async ensureCurrentMonth() {
    if (!this.currentRoot) return;
    try {
      const budgets = await this.invokeCmd<Budget[]>('ensure_month_budget', {
        rootId: this.currentRootId,
        period: currentPeriod(),
      });
      this.applyBudgets(budgets);
    } catch (e) {
      this.addLog('err', `ensure_month_budget failed: ${e}`);
    }
  }

  async load() {
    this.loading = true;
    try {
      this.data = await this.invokeCmd<AppData>('load_data', {});
      this.restoreSelection();
      await this.ensureCurrentMonth();
    } catch (e) {
      this.addLog('err', `load_data failed: ${e}`);
    }
    this.loading = false;
  }

  /** Switch to another root budget; the view resets to its current month. */
  selectRoot(id: number) {
    if (id === this.currentRootId) return;
    this.currentRootId = id;
    this.currentScope = currentPeriod();
    this.persistSelection();
    void this.ensureCurrentMonth();
  }

  setScope(scope: Scope) {
    this.currentScope = scope;
    // The period-range filter chips are 'all'-scope only; keep the state in
    // sync so leaving 'all' doesn't leave a stale filter dot behind.
    if (scope !== 'all') this.dateRange = 'all';
    this.persistSelection();
  }

  async addItem(item: { name: string; amount: number; category: string; date: string; description: string; link: string; item_type: string }) {
    try {
      // Custom sub-budgets keep their items; everything else routes by date.
      const budgetId = this.scopeType === 'custom' ? this.scopeCustomId! : this.currentRootId;
      const budgets = await this.invokeCmd<Budget[]>('add_item', {
        budgetId,
        name: item.name,
        amount: item.amount,
        category: item.category,
        date: item.date,
        description: item.description,
        link: item.link,
        itemType: item.item_type,
      });
      this.applyBudgets(budgets);
      const itemPeriod = periodOf(item.date);
      if (this.scopeType === 'month' && itemPeriod !== this.scopePeriod) {
        this.showSnackbar(`Добавлено в ${periodLabel(itemPeriod)} ✓`);
      } else {
        this.showSnackbar('Запись добавлена ✓');
      }
    } catch (e) {
      this.addLog('err', `add_item failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async updateItem(itemId: number, item: { name: string; amount: number; category: string; date: string; description: string; link: string; item_type: string }) {
    try {
      const budgets = await this.invokeCmd<Budget[]>('update_item', {
        itemId,
        name: item.name,
        amount: item.amount,
        category: item.category,
        date: item.date,
        description: item.description,
        link: item.link,
        itemType: item.item_type,
      });
      this.applyBudgets(budgets);
      const itemPeriod = periodOf(item.date);
      if (this.scopeType === 'month' && itemPeriod !== this.scopePeriod) {
        this.showSnackbar(`Перенесено в ${periodLabel(itemPeriod)} ✓`);
      } else {
        this.showSnackbar('Запись обновлена ✓');
      }
    } catch (e) {
      this.addLog('err', `update_item failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async deleteItem(itemId: number) {
    try {
      const budgets = await this.invokeCmd<Budget[]>('delete_item', { itemId });
      this.applyBudgets(budgets);
      this.showSnackbar('Запись удалена');
    } catch (e) {
      this.addLog('err', `delete_item failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async toggleCompleted(itemId: number) {
    // Un-completing an item that was materialized from a recurring rule or
    // product means undoing that occurrence: delete it and roll the source back
    // so the virtual occurrence reappears (e.g. cancelled subscription).
    const existing = this.findItem(itemId);
    if (existing?.completed && existing.source_kind) {
      await this.unmaterializeItem(itemId);
      return;
    }
    try {
      const budgets = await this.invokeCmd<Budget[]>('toggle_completed', { itemId });
      this.applyBudgets(budgets);
      const item = this.findItem(itemId);
      const label = item?.item_type === 'income' ? 'полученное' : 'оплаченное';
      this.showSnackbar(item?.completed ? `Отмечено как ${label}` : 'Возвращено в план');
    } catch (e) {
      this.addLog('err', `toggle_completed failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  /** Undo a materialized occurrence: remove the real item and revert its source. */
  async unmaterializeItem(itemId: number) {
    try {
      const result = await this.invokeCmd<UnmaterializeResult>('unmaterialize_item', { itemId });
      this.applyBudgets(result.budgets);
      if (this.data) {
        this.data.recurring = result.recurring;
        this.data.products = result.products;
      }
      this.showSnackbar('Возвращено в план');
    } catch (e) {
      this.addLog('err', `unmaterialize_item failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  openRecurringModal(rule: Recurring | null = null) {
    this.editingRecurring = rule;
    this.showRecurringModal = true;
  }

  closeRecurringModal() {
    this.showRecurringModal = false;
    this.editingRecurring = null;
  }

  async addRecurring(rule: { name: string; amount: number; category: string; item_type: string; freq: string; anchor_day: number; start_date: string; end_date: string | null; horizon: number; description: string; link: string }) {
    try {
      const recurring = await this.invokeCmd<Recurring[]>('add_recurring', {
        budgetId: this.currentRootId,
        name: rule.name,
        amount: rule.amount,
        category: rule.category,
        itemType: rule.item_type,
        freq: rule.freq,
        anchorDay: rule.anchor_day,
        startDate: rule.start_date,
        endDate: rule.end_date,
        horizon: rule.horizon,
        description: rule.description,
        link: rule.link,
      });
      if (this.data) this.data.recurring = recurring;
      this.showSnackbar('Регулярный платёж добавлен ✓');
    } catch (e) {
      this.addLog('err', `add_recurring failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async updateRecurring(id: number, rule: { name: string; amount: number; category: string; item_type: string; freq: string; anchor_day: number; start_date: string; end_date: string | null; horizon: number; description: string; link: string }) {
    try {
      const recurring = await this.invokeCmd<Recurring[]>('update_recurring', {
        id,
        name: rule.name,
        amount: rule.amount,
        category: rule.category,
        itemType: rule.item_type,
        freq: rule.freq,
        anchorDay: rule.anchor_day,
        startDate: rule.start_date,
        endDate: rule.end_date,
        horizon: rule.horizon,
        description: rule.description,
        link: rule.link,
      });
      if (this.data) this.data.recurring = recurring;
      this.showSnackbar('Регулярный платёж обновлён ✓');
    } catch (e) {
      this.addLog('err', `update_recurring failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async deleteRecurring(id: number) {
    try {
      const recurring = await this.invokeCmd<Recurring[]>('delete_recurring', { id });
      if (this.data) this.data.recurring = recurring;
      this.showSnackbar('Регулярный платёж удалён');
    } catch (e) {
      this.addLog('err', `delete_recurring failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  /** Materialize a virtual occurrence into a real completed item and advance its source. */
  async materializeOccurrence(source: ItemSource) {
    if (source.kind === 'recurring') {
      try {
        const result = await this.invokeCmd<MaterializeResult>('materialize_recurring', {
          id: source.id,
          date: source.date,
        });
        this.applyBudgets(result.budgets);
        if (this.data) this.data.recurring = result.recurring;
        this.showSnackbar('Записано ✓');
      } catch (e) {
        this.addLog('err', `materialize_recurring failed: ${e}`);
        this.showSnackbar('Ошибка: ' + String(e));
      }
      return;
    }
    // deposit | loan
    const occ = this.productOccurrences.find(
      o => o.source?.id === source.id && o.source?.date === source.date,
    );
    if (!occ || !occ.source) return;
    try {
      const result = await this.invokeCmd<ProductResult>('materialize_product', {
        id: source.id,
        date: source.date,
        amount: occ.amount,
        principalPortion: occ.source.principalPortion ?? 0,
        itemType: occ.item_type,
        closes: occ.source.closes ?? false,
      });
      this.applyBudgets(result.budgets);
      if (this.data) this.data.products = result.products;
      this.showSnackbar('Записано ✓');
    } catch (e) {
      this.addLog('err', `materialize_product failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  openProductModal(productOrKind: Product | 'deposit' | 'loan' | 'mortgage' = 'deposit') {
    if (typeof productOrKind === 'string') {
      this.editingProduct = null;
      this.newProductKind = productOrKind;
    } else {
      this.editingProduct = productOrKind;
      this.newProductKind = productOrKind.kind;
    }
    this.showProductModal = true;
  }

  closeProductModal() {
    this.showProductModal = false;
    this.editingProduct = null;
  }

  async addProduct(p: { kind: string; name: string; principal: number; annual_rate_bps: number; term_months: number; start_date: string; payment_model: string; early_rate_bps: number | null; horizon: number; category: string; down_payment: number; description: string; link: string }) {
    try {
      const result = await this.invokeCmd<ProductResult>('add_product', {
        budgetId: this.currentRootId,
        kind: p.kind,
        name: p.name,
        principal: p.principal,
        annualRateBps: p.annual_rate_bps,
        termMonths: p.term_months,
        startDate: p.start_date,
        paymentModel: p.payment_model,
        earlyRateBps: p.early_rate_bps,
        horizon: p.horizon,
        category: p.category,
        downPayment: p.down_payment,
        description: p.description,
        link: p.link,
      });
      this.applyBudgets(result.budgets);
      if (this.data) this.data.products = result.products;
      const label = p.kind === 'deposit' ? 'Вклад открыт ✓' : p.kind === 'mortgage' ? 'Ипотека добавлена ✓' : 'Кредит добавлен ✓';
      this.showSnackbar(label);
    } catch (e) {
      this.addLog('err', `add_product failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async updateProduct(id: number, p: { name: string; principal: number; annual_rate_bps: number; term_months: number; start_date: string; payment_model: string; early_rate_bps: number | null; horizon: number; category: string; down_payment: number; description: string; link: string }) {
    try {
      const products = await this.invokeCmd<Product[]>('update_product', {
        id,
        name: p.name,
        principal: p.principal,
        annualRateBps: p.annual_rate_bps,
        termMonths: p.term_months,
        startDate: p.start_date,
        paymentModel: p.payment_model,
        earlyRateBps: p.early_rate_bps,
        horizon: p.horizon,
        category: p.category,
        downPayment: p.down_payment,
        description: p.description,
        link: p.link,
      });
      if (this.data) this.data.products = products;
      this.showSnackbar('Продукт обновлён ✓');
    } catch (e) {
      this.addLog('err', `update_product failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async deleteProduct(id: number) {
    try {
      const products = await this.invokeCmd<Product[]>('delete_product', { id });
      if (this.data) this.data.products = products;
      this.showSnackbar('Продукт удалён');
    } catch (e) {
      this.addLog('err', `delete_product failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async loanExtraPayment(id: number, amount: number, date: string) {
    try {
      const result = await this.invokeCmd<ProductResult>('loan_extra_payment', { id, amount, date });
      this.applyBudgets(result.budgets);
      if (this.data) this.data.products = result.products;
      this.showSnackbar('Досрочный платёж внесён ✓');
    } catch (e) {
      this.addLog('err', `loan_extra_payment failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async closeDeposit(id: number, date: string, payout: number) {
    try {
      const result = await this.invokeCmd<ProductResult>('close_deposit', { id, date, payout });
      this.applyBudgets(result.budgets);
      if (this.data) this.data.products = result.products;
      this.showSnackbar('Вклад закрыт ✓');
    } catch (e) {
      this.addLog('err', `close_deposit failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async createBudget(name: string, limit: number, icon: string) {
    try {
      const before = new Set(this.roots.map(r => r.id));
      const budgets = await this.invokeCmd<Budget[]>('create_budget', { name, limit, icon });
      this.applyBudgets(budgets);
      const created = this.roots.find(r => !before.has(r.id));
      if (created) this.selectRoot(created.id);
      this.showSnackbar('Бюджет создан ✓');
    } catch (e) {
      this.addLog('err', `create_budget failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  /** Create a custom (themed) sub-budget under the current root. */
  async createSubBudget(name: string, limit: number, icon: string, reflectInMonths: boolean) {
    try {
      const budgets = await this.invokeCmd<Budget[]>('create_sub_budget', {
        rootId: this.currentRootId,
        name,
        limit,
        icon,
        reflectInMonths,
      });
      this.applyBudgets(budgets);
      this.showSnackbar('Под-бюджет создан ✓');
    } catch (e) {
      this.addLog('err', `create_sub_budget failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async deleteBudget(budgetId: number) {
    try {
      const budgets = await this.invokeCmd<Budget[]>('delete_budget', { budgetId });
      this.applyBudgets(budgets);
      if (budgetId === this.currentRootId) {
        const first = this.roots[0];
        if (first) this.selectRoot(first.id);
      } else if (this.scopeCustomId === budgetId) {
        this.setScope(currentPeriod());
      }
      this.showSnackbar('Бюджет удалён');
    } catch (e) {
      this.addLog('err', `delete_budget failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async updateBudget(budgetId: number, name: string, limit: number, icon: string, reflectInMonths?: boolean) {
    try {
      const target = this.data?.budgets.find(b => b.id === budgetId);
      const budgets = await this.invokeCmd<Budget[]>('update_budget', {
        budgetId,
        name,
        limit,
        icon,
        reflectInMonths: reflectInMonths ?? target?.reflect_in_months ?? false,
      });
      this.applyBudgets(budgets);
      this.showSnackbar('Бюджет обновлён ✓');
    } catch (e) {
      this.addLog('err', `update_budget failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  /** Set the limit of one month sub-budget (auto-creating its row if needed). */
  async updateMonthLimit(period: string, limit: number) {
    try {
      let month = this.monthBudgets.find(b => b.period === period);
      if (!month) {
        const budgets = await this.invokeCmd<Budget[]>('ensure_month_budget', {
          rootId: this.currentRootId,
          period,
        });
        this.applyBudgets(budgets);
        month = this.monthBudgets.find(b => b.period === period);
      }
      if (!month) throw new Error('месяц не найден');
      const budgets = await this.invokeCmd<Budget[]>('update_budget', {
        budgetId: month.id,
        name: month.name,
        limit,
        icon: month.icon,
        reflectInMonths: false,
      });
      this.applyBudgets(budgets);
      this.showSnackbar('Лимит месяца обновлён ✓');
    } catch (e) {
      this.addLog('err', `update month limit failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async addCategory(key: string, name: string, icon: string, color: string) {
    try {
      const cats = await this.invokeCmd<Category[]>('add_category', { key, name, icon, color });
      if (this.data) this.data.categories = cats;
      this.showSnackbar('Категория добавлена ✓');
    } catch (e) {
      this.addLog('err', `add_category failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async updateCategory(key: string, name: string, icon: string, color: string) {
    try {
      const cats = await this.invokeCmd<Category[]>('update_category', { key, name, icon, color });
      if (this.data) this.data.categories = cats;
      this.showSnackbar('Категория обновлена ✓');
    } catch (e) {
      this.addLog('err', `update_category failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async deleteCategory(key: string) {
    try {
      const cats = await this.invokeCmd<Category[]>('delete_category', { key });
      if (this.data) this.data.categories = cats;
      this.showSnackbar('Категория удалена');
    } catch (e) {
      this.addLog('err', `delete_category failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async exportCsv(): Promise<string | null> {
    try {
      return await this.invokeCmd<string>('export_csv', { budgetId: this.currentRootId });
    } catch (e) {
      this.addLog('err', `export_csv failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
      return null;
    }
  }

  async exportJson(): Promise<string | null> {
    try {
      return await this.invokeCmd<string>('export_json', {});
    } catch (e) {
      this.addLog('err', `export_json failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
      return null;
    }
  }

  async importJson(json: string) {
    try {
      this.data = await this.invokeCmd<AppData>('import_json', { json });
      // Imported data may have entirely different roots — revalidate selection.
      this.currentRootId = this.roots[0]?.id ?? 1;
      this.currentScope = currentPeriod();
      this.persistSelection();
      await this.ensureCurrentMonth();
      this.showSnackbar('Данные импортированы ✓');
    } catch (e) {
      this.addLog('err', `import_json failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  showSnackbar(msg: string, durationMs?: number) {
    if (this.snackbarTimer) clearTimeout(this.snackbarTimer);
    this.snackbarMsg = msg;
    this.snackbarShow = true;
    // Longer duration for error messages
    const duration = durationMs ?? (msg.startsWith('Ошибка') ? 4000 : 2500);
    this.snackbarTimer = setTimeout(() => { this.snackbarShow = false; }, duration);
  }

  showConfirm(msg: string, cb: () => void, label: string = 'Удалить') {
    this.confirmMsg = msg;
    this.confirmCallback = cb;
    this.confirmLabel = label;
    this.confirmShow = true;
  }

  closeConfirm() {
    this.confirmShow = false;
    this.confirmMsg = '';
    this.confirmCallback = null;
    this.confirmLabel = 'Удалить';
  }
}

export const store = new BudgetStore();
