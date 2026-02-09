<script lang="ts">
	const { data } = $props();
</script>

<section class="search-top">
	<h1>Search: {data.query}</h1>
</section>

<section class="search-results">
	<div class="profiles">
		<p>Profiles</p>
	</div>
	<div class="posts">
		<p>Posts</p>
	</div>
</section>

<style>
</style>
