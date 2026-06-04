<template>
  <div>
    <h1>{{ store.name }}</h1>
    <p>Count: {{ store.count }}</p>
    <p>Double: {{ store.double }}</p>
    <p>Items: {{ store.itemCount }}</p>
    <button @click="handleIncrement">+1</button>
    <button @click="handleDecrement">-1</button>
    <button @click="handleAddItem">Add Item</button>
    <button @click="handleReset">Reset</button>
  </div>
</template>

<script setup lang="ts">
import { useCounterStore } from './counterStore'

const store = useCounterStore()

function handleIncrement() {
  store.increment()
}

function handleDecrement() {
  store.decrement()
}

function handleAddItem() {
  store.addItem('new item')
}

function handleReset() {
  store.reset()
}
</script>
