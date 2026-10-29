// Nuxt UI theme. Colors are semantic: primary is the brand purple (defined in assets/main.css), and the
// review ratings use the status colors: Again = error, Hard = warning, Good = success, Easy = info.
export default defineAppConfig({
  ui: {
    colors: {
      primary: 'brand',
      neutral: 'gray',
      success: 'green',
      info: 'blue',
      warning: 'yellow',
      error: 'red',
    },
    // Controls at the default size (xl) are 44px tall, a comfortable touch target.
    button: { variants: { size: { xl: { base: 'px-4 py-2.5' } } } },
    input: { variants: { size: { xl: { base: 'py-2.5' } } } },
    select: { variants: { size: { xl: { base: 'py-2.5' } } } },
    card: { slots: { root: 'shadow-sm' } },
  },
});
