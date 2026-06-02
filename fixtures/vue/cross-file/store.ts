import { ref } from 'vue'

export const count = ref(0)
export const name = ref('hello')

export function increment() {
  count.value++
}

export function reset() {
  count.value = 0
  name.value = ''
}

export function fetchData() {
  return { count: count.value, name: name.value }
}
