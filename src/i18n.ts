import { createI18n } from 'vue-i18n'

const messages = {
  en: {
    message: {
      hello: 'hello world',
      dashboard: 'Dashboard',
      settings: 'Settings',
      theme: 'Theme',
      language: 'Language',
      light: 'Light',
      dark: 'Dark',
      system: 'System',
      autoStart: 'Auto-start on boot',
      screenshotInterval: 'Screenshot Interval (mins)',
      screenshotLocation: 'Screenshot Location',
      backupFrequency: 'Backup Frequency',
      backupLocation: 'Backup Location',
      daily: 'Daily',
      weekly: 'Weekly',
      never: 'Never',
      todaySummary: 'Today\'s Summary',
      activeTime: 'Active Time',
      idleTime: 'Idle Time',
      topApps: 'Top Apps'
    }
  },
  bn: {
    message: {
      hello: 'ওহে বিশ্ব',
      dashboard: 'ড্যাশবোর্ড',
      settings: 'সেটিংস',
      theme: 'থিম',
      language: 'ভাষা',
      light: 'লাইট',
      dark: 'ডার্ক',
      system: 'সিস্টেম',
      autoStart: 'বুট করার সময় স্বয়ংক্রিয়ভাবে শুরু করুন',
      screenshotInterval: 'স্ক্রিনশট ব্যবধান (মিনিট)',
      screenshotLocation: 'স্ক্রিনশট অবস্থান',
      backupFrequency: 'ব্যাকআপ ফ্রিকোয়েন্সি',
      backupLocation: 'ব্যাকআপ অবস্থান',
      daily: 'প্রতিদিন',
      weekly: 'সাপ্তাহিক',
      never: 'কখনও না',
      todaySummary: 'আজকের সারাংশ',
      activeTime: 'সক্রিয় সময়',
      idleTime: 'অলস সময়',
      topApps: 'শীর্ষ অ্যাপস'
    }
  }
}

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  fallbackLocale: 'en',
  messages,
})

export default i18n
