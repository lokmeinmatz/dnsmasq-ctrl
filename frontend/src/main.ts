import './style.scss'
import dayjs from 'dayjs'

import { CategoryScale, Chart, LinearScale, LineController, LineElement, PointElement } from 'chart.js'


Chart.register(
  LineController,
  LineElement,
  LinearScale,
  CategoryScale,
  PointElement
)

interface DynamicData {
  numHits: number,
  numTotal: number,
  percentFromCache?: number
  topQueryDomains:{ [domain: string]: number }
  topQueryTypes:{ [type: string]: number }
  topQuerySources:{ [source: string]: number }
  unknownDomains:{ [source: string]: number }
  lookupTimeline: { start: string, requests: number }[]
}

function renderGridList<T>(base: HTMLElement, data: T[], func: (d: T) => HTMLElement[]) {
  for (const d of data) {
    const elements = func(d)
    base.append(...elements)
  }
}


function mapSort(d: Record<string, number>): [string, number][] {
  return Object.entries(d).sort((a, b) => b[1] - a[1])
}

function createEl(innerHtml: string): HTMLDivElement {
  const el = document.createElement('div')
  el.innerHTML = innerHtml
  return el
}

function main() {
  fetch('/api/static').then(res => res.json()).then(staticData => {
    document.getElementById('version')!.innerText = 'Version: ' + staticData.version  
    document.querySelector<HTMLSpanElement>('#cache-size > span')!.innerText = staticData.cacheSize
    const $nameServers = document.getElementById('nameservers')! as HTMLUListElement
    
    renderGridList($nameServers, staticData.nameServers as string[], (server) => [createEl(server)])
  })

  fetch('/api/dynamic').then(res => res.json()).then((dynamicData: DynamicData) => {

    const $cache = document.getElementById('requests') as HTMLElement

    $cache.querySelector('#request-text')!.textContent = `hits: ${dynamicData.numHits} of ${dynamicData.numTotal}`;

    ($cache.querySelector('.bg') as HTMLElement).style.width = ((dynamicData.percentFromCache ?? 0) * 100) + '%'

    const $domains = document.getElementById('domains')!
    const $clients = document.getElementById('clients')!
    const $types = document.getElementById('type')!
    const $nxdomains = document.getElementById('nxdomains')!
    const $timeline = document.getElementById('timeline')! as HTMLCanvasElement
    
    renderGridList($domains, mapSort(dynamicData.topQueryDomains).slice(0, 50), (domain) => [createEl(domain[0]), createEl(domain[1].toString())])
    renderGridList($clients, mapSort(dynamicData.topQuerySources).slice(0, 50), (source) => [createEl(source[0]), createEl(source[1].toString())])
    renderGridList($types, mapSort(dynamicData.topQueryTypes), (type) => [createEl(type[0]), createEl(type[1].toString())])
    renderGridList($nxdomains, mapSort(dynamicData.unknownDomains), (type) => [createEl(type[0]), createEl(type[1].toString())])
  

    let data = dynamicData.lookupTimeline

    if (data.length < 24) {
      let date = dayjs(data[data.length - 1].start ?? dayjs())
      for (let i = data.length; i < 24; i++) {
        date = date.subtract(1, 'hour')
        data.unshift({ requests: 0, start: date.toISOString() })
      }
    }
  
    new Chart($timeline.getContext('2d')!, {
      type: 'line',
      data: {
        labels: dynamicData.lookupTimeline.map(bucket => dayjs(bucket.start).format('HH:mm')),
        datasets: [{
          data: dynamicData.lookupTimeline.map(bucket => bucket.requests)
        }]
      }
    })
  })
}

if (document.readyState !== 'complete') {
  console.log('not ready yet')
  document.addEventListener('DOMContentLoaded', main)
} else {
  console.log('direct init')
  main()
}