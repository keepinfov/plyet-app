import { invoke } from '@tauri-apps/api/core';
import { theme } from 'reglass-material/theme';
import type { AppData, Budget, Item, Category, Recurring, Product, ItemSource, MaterializeResult, ProductResult, UnmaterializeResult } from '$lib/types';
import { nextOccurrences, productOccurrences } from '$lib/finance';

type LogLevel = 'ok' | 'err' | 'warn' | 'info';

interface LogEntry {
  ts: string;
  level: LogLevel;
  msg: string;
}

class BudgetStore {
  data = $state<AppData | null>(null);
  loading = $state(true);
  currentBudgetId = $state<number>(1);
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

  currentBudget = $derived(
    this.data?.budgets.find(b => b.id === this.currentBudgetId) ?? null
  );

  /** Recurring rules belonging to the current budget. */
  recurringForBudget = $derived(
    this.data?.recurring.filter(r => r.budget_id === this.currentBudgetId) ?? []
  );

  /** Products (deposits/loans) belonging to the current budget. */
  productsForBudget = $derived(
    this.data?.products.filter(p => p.budget_id === this.currentBudgetId) ?? []
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
        });
      });
    }
    return out;
  });

  /** Real items plus virtual occurrences — the source for the feed/projections. */
  feedItems = $derived([
    ...(this.currentBudget?.items ?? []),
    ...this.virtualOccurrences,
    ...this.productOccurrences,
  ]);

  filteredItems = $derived.by(() => {
    const budget = this.currentBudget;
    if (!budget) return [];
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

    // Date range filter
    if (this.dateRange !== 'all') {
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

  private mergeBudget(budget: Budget) {
    if (!this.data) return;
    const idx = this.data.budgets.findIndex(b => b.id === budget.id);
    if (idx >= 0) {
      this.data.budgets[idx] = budget;
    } else {
      this.data.budgets = [...this.data.budgets, budget];
    }
  }

  async load() {
    this.loading = true;
    try {
      this.data = await this.invokeCmd<AppData>('load_data', {});
      if (this.data.budgets.length > 0) this.currentBudgetId = this.data.budgets[0].id;
    } catch (e) {
      this.addLog('err', `load_data failed: ${e}`);
    }
    this.loading = false;
  }

  async addItem(item: { name: string; amount: number; category: string; date: string; description: string; link: string; item_type: string }) {
    try {
      const budget = await this.invokeCmd<Budget>('add_item', {
        budgetId: this.currentBudgetId,
        name: item.name,
        amount: item.amount,
        category: item.category,
        date: item.date,
        description: item.description,
        link: item.link,
        itemType: item.item_type,
      });
      this.mergeBudget(budget);
      this.showSnackbar('Запись добавлена ✓');
    } catch (e) {
      this.addLog('err', `add_item failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async updateItem(itemId: number, item: { name: string; amount: number; category: string; date: string; description: string; link: string; item_type: string }) {
    try {
      const budget = await this.invokeCmd<Budget>('update_item', {
        budgetId: this.currentBudgetId,
        itemId,
        name: item.name,
        amount: item.amount,
        category: item.category,
        date: item.date,
        description: item.description,
        link: item.link,
        itemType: item.item_type,
      });
      this.mergeBudget(budget);
      this.showSnackbar('Запись обновлена ✓');
    } catch (e) {
      this.addLog('err', `update_item failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async deleteItem(itemId: number) {
    try {
      const budget = await this.invokeCmd<Budget>('delete_item', { budgetId: this.currentBudgetId, itemId });
      this.mergeBudget(budget);
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
    const existing = this.currentBudget?.items.find(i => i.id === itemId);
    if (existing?.completed && existing.source_kind) {
      await this.unmaterializeItem(itemId);
      return;
    }
    try {
      const budget = await this.invokeCmd<Budget>('toggle_completed', { budgetId: this.currentBudgetId, itemId });
      this.mergeBudget(budget);
      const item = budget.items.find(i => i.id === itemId);
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
      const result = await this.invokeCmd<UnmaterializeResult>('unmaterialize_item', {
        budgetId: this.currentBudgetId,
        itemId,
      });
      this.mergeBudget(result.budget);
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
        budgetId: this.currentBudgetId,
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
        this.mergeBudget(result.budget);
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
      this.mergeBudget(result.budget);
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
        budgetId: this.currentBudgetId,
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
      this.mergeBudget(result.budget);
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
      this.mergeBudget(result.budget);
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
      this.mergeBudget(result.budget);
      if (this.data) this.data.products = result.products;
      this.showSnackbar('Вклад закрыт ✓');
    } catch (e) {
      this.addLog('err', `close_deposit failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async createBudget(name: string, limit: number, icon: string) {
    try {
      const budget = await this.invokeCmd<Budget>('create_budget', { name, limit, icon });
      this.mergeBudget(budget);
      this.currentBudgetId = budget.id;
      this.showSnackbar('Бюджет создан ✓');
    } catch (e) {
      this.addLog('err', `create_budget failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async deleteBudget(budgetId: number) {
    try {
      await this.invokeCmd<void>('delete_budget', { budgetId });
      if (this.data) {
        this.data.budgets = this.data.budgets.filter(b => b.id !== budgetId);
        if (this.data.budgets.length > 0) this.currentBudgetId = this.data.budgets[0].id;
      }
      this.showSnackbar('Бюджет удалён');
    } catch (e) {
      this.addLog('err', `delete_budget failed: ${e}`);
      this.showSnackbar('Ошибка: ' + String(e));
    }
  }

  async updateBudget(budgetId: number, name: string, limit: number, icon: string) {
    try {
      const budget = await this.invokeCmd<Budget>('update_budget', { budgetId, name, limit, icon });
      this.mergeBudget(budget);
      this.showSnackbar('Бюджет обновлён ✓');
    } catch (e) {
      this.addLog('err', `update_budget failed: ${e}`);
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
      return await this.invokeCmd<string>('export_csv', { budgetId: this.currentBudgetId });
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
      if (this.data.budgets.length > 0) this.currentBudgetId = this.data.budgets[0].id;
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
