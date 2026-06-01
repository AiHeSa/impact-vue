<template>
  <div>
    <h1>Parent</h1>
    <p>Count: {{ count }}</p>
    <ChildComponent :count="count" @update="handleUpdate" />
  </div>
</template>

<script>
import ChildComponent from './Child.vue'

export default {
  components: {
    ChildComponent
  },
  data() {
    return {
      count: 0
    }
  },
  methods: {
    handleUpdate(value) {
      this.count = value
    }
  }
}
</script>
