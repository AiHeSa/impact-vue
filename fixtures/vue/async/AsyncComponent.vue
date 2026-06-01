<template>
  <div>
    <p>{{ data }}</p>
    <p>{{ error }}</p>
    <button @click="fetchData">Fetch</button>
  </div>
</template>

<script>
export default {
  data() {
    return {
      data: null,
      error: null,
      loading: false
    }
  },
  methods: {
    async fetchData() {
      this.loading = true
      try {
        const res = await api.getData()
        this.data = res.data
      } catch (e) {
        this.error = e.message
      } finally {
        this.loading = false
      }
    },
    loadData() {
      api.getData().then(res => {
        this.data = res.data
      }).catch(e => {
        this.error = e.message
      })
    }
  }
}
</script>
