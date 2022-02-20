import './style.scss'

interface DynamicData {
  numHits: number,
  numTotal: number,
  percentFromCache?: number
  topQueryDomains:{ [domain: string]: number }
  topQueryTypes:{ [type: string]: number }
  topQuerySources:{ [source: string]: number }
}

function renderList<T>(base: HTMLElement, data: T[], func: (el: HTMLLIElement, d: T) => void) {
  for (const d of data) {
    const el = document.createElement('li')
    func(el, d)
    base.append(el)
  }
}


function mapSort(d: Record<string, number>): [string, number][] {
  return Object.entries(d).sort((a, b) => b[1] - a[1])
}

function main() {
  fetch('/api/static').then(res => res.json()).then(staticData => {
    document.getElementById('version')!.innerText = 'Version: ' + staticData.version  
    document.querySelector<HTMLSpanElement>('#cache-size > span')!.innerText = staticData.cacheSize
    const $nameServers = document.getElementById('nameservers')! as HTMLUListElement
    
    renderList($nameServers, staticData.nameServers as string[], (li, server) => li.innerText = server)
  })

  fetch('/api/dynamic').then(res => res.json()).then((dynamicData: DynamicData) => {

    const $cache = document.getElementById('requests') as HTMLElement

    $cache.querySelector('#request-text')!.textContent = `hits: ${dynamicData.numHits} of ${dynamicData.numTotal}`;

    ($cache.querySelector('.bg') as HTMLElement).style.width = ((dynamicData.percentFromCache ?? 0) * 100) + '%'

    const $domains = document.getElementById('domains')!
    const $clients = document.getElementById('clients')!
    const $types = document.getElementById('type')!
    
    renderList($domains, mapSort(dynamicData.topQueryDomains), (li, domain) => li.innerHTML = `<span>${domain[0]}</span><span>${domain[1]}</span>`)
    renderList($clients, mapSort(dynamicData.topQuerySources), (li, source) => li.innerHTML = `<span>${source[0]}</span><span>${source[1]}</span>`)
    renderList($types, mapSort(dynamicData.topQueryTypes), (li, type) => li.innerHTML = `<span>${type[0]}</span><span>${type[1]}</span>`)
  })
}

if (document.readyState !== 'complete') {
  console.log('not ready yet')
  document.addEventListener('DOMContentLoaded', main)
} else {
  console.log('direct init')
  main()
}