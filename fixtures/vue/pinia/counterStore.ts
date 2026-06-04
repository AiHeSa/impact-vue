import { defineStore } from 'pinia'

export const useCounterStore = defineStore('counter', {
  state: () => ({
    count: 0,
    name: 'Counter',
    items: [] as string[],
  }),
  getters: {
    double: (state) => state.count * 2,
    itemCount: (state) => state.items.length,
  },
  actions: {
    increment() {
      this.count++
    },
    decrement() {
      this.count--
    },
    addItem(item: string) {
      this.items.push(item)
    },
    reset() {
      this.count = 0
      this.items = []
    },
  },
})
