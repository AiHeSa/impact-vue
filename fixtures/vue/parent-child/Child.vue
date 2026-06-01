<template>
  <div>
    <h2>Child</h2>
    <p>Count: {{ count }}</p>
    <p>Double: {{ double }}</p>
    <button @click="increment">+1</button>
  </div>
</template>

<script>
export default {
  props: ['count'],
  computed: {
    double() {
      return this.count * 2
    }
  },
  methods: {
    increment() {
      this.$emit('update', this.count + 1)
    }
  }
}
</script>
