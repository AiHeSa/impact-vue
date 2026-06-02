<template>
  <div>
    <p>{{ count }}</p>
    <p>{{ name }}</p>
    <button @click="handleIncrement">+1</button>
    <button @click="handleReset">Reset</button>
  </div>
</template>

<script setup lang="ts">
import { count, name, increment, reset } from './store'

function handleIncrement() {
  increment()
}

function handleReset() {
  reset()
}
</script>
